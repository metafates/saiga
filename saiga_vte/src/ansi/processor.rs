use std::iter;

use log::debug;

use super::{
    c0,
    handler::{Charset, CharsetIndex, Column, Direction, Handler, Line, Position, Rgb},
};
use crate::{
    ansi::handler::{
        Attribute, Color, Hyperlink, LineClearMode, Mode, NamedColor, NamedPrivateMode,
        PrivateMode, ScreenClearMode,
    },
    param::{Param, Params, Subparam},
};
use crate::{param, utf8, Executor, MAX_INTERMEDIATES};

#[derive(Default)]
pub struct Processor {
    parser: crate::Parser,
}

impl Processor {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn advance<H: Handler>(&mut self, handler: &mut H, bytes: &[u8]) {
        let mut executor = HandlerExecutor::new(handler);

        self.parser.advance(&mut executor, bytes);
    }
}

struct HandlerExecutor<'a, H: Handler> {
    handler: &'a mut H,
}

impl<'a, H: Handler + 'a> HandlerExecutor<'a, H> {
    fn new<'b>(handler: &'b mut H) -> HandlerExecutor<'b, H> {
        HandlerExecutor { handler }
    }
}

impl<'a, H: Handler> Executor for HandlerExecutor<'a, H> {
    fn print(&mut self, c: char) {
        self.handler.put_char(c);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            c0::HT => self.handler.put_tab(),
            c0::CR => self.handler.carriage_return(),
            c0::BS => self.handler.backspace(),
            c0::BEL => self.handler.ring_bell(),
            c0::LF | c0::VT | c0::FF => self.handler.linefeed(),
            c0::SI => self
                .handler
                .set_active_charset(super::handler::CharsetIndex::G0),
            c0::SO => self
                .handler
                .set_active_charset(super::handler::CharsetIndex::G1),
            c0::SUB => self.handler.substitute(),
            _ => debug!("[Unhandled execute] byte={byte:02x}"),
        }
    }

    fn put(&mut self, byte: u8) {
        debug!("[Unhandled put] byte={byte:02x}")
    }

    fn hook(
        &mut self,
        params: &crate::param::Params,
        intermediates: &[u8],
        ignore: bool,
        action: char,
    ) {
        debug!("[Unhandled hook] params={params:?} intermediates={intermediates:?} ignore={ignore:?} action={action:?}");
    }

    fn unhook(&mut self) {
        debug!("[Unhandled unhook]");
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], bell_terminated: bool) {
        macro_rules! unhandled {
            () => {{
                debug!("[Unhandled OSC] params={params:?}, bell_terminated={bell_terminated:?}",);
            }};
        }

        static URI_PREFIXES: [&[u8]; 5] =
            [b"https://", b"http://", b"file://", b"mailto://", b"ftp://"];

        match params {
            // set window title
            [b"0" | b"2", title @ ..] => {
                let title = title.join(&param::PARAM_SEPARATOR);

                if let Ok(title) = utf8::from_utf8(&title) {
                    self.handler.set_title(title);
                }
            }

            // Change color number
            [b"4", params @ ..] if !params.is_empty() && params.len() % 2 == 0 => {}

            // Create a hyperlink to uri using params.
            [b"8", params, uri] if URI_PREFIXES.into_iter().any(|p| uri.starts_with(p)) => {
                // Link parameters are in format of `key1=value1:key2=value2`. Currently only key
                // `id` is defined.
                let id = params
                    .split(|&b| b == b':')
                    .find_map(|kv| kv.strip_prefix(b"id="))
                    .and_then(|kv| utf8::from_utf8(kv).ok().map(|e| e.to_owned()));

                let uri = utf8::from_utf8(uri).unwrap_or_default().to_string();

                self.handler.put_hyperlink(Hyperlink { id, uri });
            }

            // Set or query default foreground color.
            [b"10", param] => {}

            // Set or query default background color.
            [b"11", param] => {}

            // Set or query default cursor color.
            [b"12", param] => {}

            // TODO: cursor shape and style

            // Set or query clipboard
            [b"52", clipboard, payload] => {
                let clipboard = clipboard.first().unwrap_or(&b'c');

                match *payload {
                    b"?" => self.handler.report_clipboard(*clipboard),
                    base64 => self.handler.set_clipboard(*clipboard, base64),
                }
            }

            // Reset color number `color` to themed color.
            [b"104", [color]] => {}

            // Restore default foreground to themed color.
            [b"110"] => {}

            // Restore default background to themed color.
            [b"111"] => {}

            // Restore default cursor to themed color.
            [b"112"] => {}

            _ => {
                unhandled!()
            }
        }
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], ignore: bool, byte: u8) {
        macro_rules! unhandled {
            () => {{
                debug!("[Unhandled ESC] intermediates={intermediates:?} ignore={ignore:?} byte={byte:02x}");
            }};
        }

        match (byte, intermediates) {
            (b'0', [index]) => {
                let Ok(index): Result<CharsetIndex, ()> = (*index).try_into() else {
                    unhandled!();
                    return;
                };

                self.handler
                    .set_charset_index(index, Charset::SpecialCharacterAndLineDrawing);
            }
            (b'B', [index]) => {
                let Ok(index): Result<CharsetIndex, ()> = (*index).try_into() else {
                    unhandled!();
                    return;
                };

                self.handler.set_charset_index(index, Charset::Ascii);
            }
            (b'D', []) => self.handler.linefeed(),
            (b'E', []) => {
                self.handler.linefeed();
                self.handler.carriage_return();
            }
            (b'Z', []) => self.handler.report_terminal(),
            (b'c', []) => self.handler.reset_state(),
            (b'7', []) => self.handler.save_cursor_position(),
            (b'8', []) => self.handler.restore_cursor_position(),

            // String terminator, do nothing (parser handles as string terminator).
            (b'\\', []) => (),

            _ => unhandled!(),
        }
    }

    fn csi_dispatch(
        &mut self,
        params: &crate::param::Params,
        intermediates: &[u8],
        ignore: bool,
        action: char,
    ) {
        macro_rules! unhandled {
            () => {{
                debug!("[Unhandled CSI] action={action:?}, params={params:?}, intermediates={intermediates:?}");
            }};
        }

        if ignore || intermediates.len() > MAX_INTERMEDIATES {
            unhandled!();
            return;
        }

        let mut params_iter = params.as_slice().into_iter();
        let mut next_param_or = |default: u16| match params_iter.next() {
            Some(param) => match param.as_slice().first() {
                Some(subparam) if *subparam != 0 => *subparam,
                _ => default,
            },
            _ => default,
        };

        match (action, intermediates) {
            ('@', []) => self.handler.put_blank(next_param_or(1).into()),
            ('A', []) => self
                .handler
                .move_cursor(Direction::Up, next_param_or(1).into(), false),
            ('B' | 'e', []) => {
                self.handler
                    .move_cursor(Direction::Down, next_param_or(1).into(), false)
            }
            ('C' | 'a', []) => {
                self.handler
                    .move_cursor(Direction::Right, next_param_or(1).into(), false)
            }
            ('c', _) => self.handler.report_terminal(), // TODO: pass intermediates?
            ('D', []) => self
                .handler
                .move_cursor(Direction::Left, next_param_or(1).into(), false),
            ('d', []) => self.handler.set_cursor_line(next_param_or(1) as Line - 1),
            ('E', []) => self
                .handler
                .move_cursor(Direction::Down, next_param_or(1).into(), true),
            ('F', []) => self
                .handler
                .move_cursor(Direction::Up, next_param_or(1).into(), true),
            ('G' | '`', []) => self
                .handler
                .set_cursor_column(next_param_or(1) as Column - 1),
            ('H' | 'f', []) => {
                let line = next_param_or(1) as Line - 1;
                let column = next_param_or(1) as Column - 1;

                self.handler.set_cursor_position(Position { line, column });
            }
            ('h', []) => {
                for param in params_iter.map(|p| p.as_slice()[0]) {
                    self.handler.set_mode(Mode::new(param));
                }
            }
            ('h', [b'?']) => {
                for param in params_iter.map(|p| p.as_slice()[0]) {
                    if param == NamedPrivateMode::SyncUpdate as u16 {
                        // TODO
                    }

                    self.handler.set_private_mode(PrivateMode::new(param));
                }
            }
            ('J', []) => {
                let mode = match next_param_or(0) {
                    0 => ScreenClearMode::Below,
                    1 => ScreenClearMode::Above,
                    2 => ScreenClearMode::All,
                    3 => ScreenClearMode::Saved,
                    _ => return,
                };

                self.handler.clear_screen(mode);
            }
            ('K', []) => {
                let mode = match next_param_or(0) {
                    0 => LineClearMode::Right,
                    1 => LineClearMode::Left,
                    2 => LineClearMode::All,
                    _ => return,
                };

                self.handler.clear_line(mode);
            }
            ('M', []) => self.handler.delete_lines(next_param_or(1).into()),
            ('m', []) => {
                if params.is_empty() {
                    self.handler.set_attribute(Attribute::Reset);
                    return;
                }

                for attribtue in attrs_from_sgr_parameters(params) {
                    self.handler.set_attribute(attribtue);
                }
            }
            ('P', []) => self.handler.delete_chars(next_param_or(1).into()),
            ('p', [b'$']) => self.handler.report_mode(Mode::new(next_param_or(0))),
            ('u', [b'?']) => self.handler.report_keyboard_mode(),
            ('u', []) => self.handler.restore_cursor_position(),
            ('X', []) => self.handler.erase_chars(next_param_or(1).into()),

            // TODO: rest
            _ => unhandled!(),
        }
    }
}

fn attrs_from_sgr_parameters(params: &Params) -> Vec<Attribute> {
    use Attribute::*;

    let mut attributes = Vec::with_capacity(params.len());
    let params = &mut params.as_slice().iter();

    while let Some(param) = params.next() {
        let attribute = match param.as_slice() {
            [0] => Some(Reset),
            [1] => Some(Bold),
            [2] => Some(Dim),
            [3] => Some(Italic),
            [4, 0] => Some(CancelUnderline),
            [4, 2] => Some(DoubleUnderline),
            [4, 3] => Some(Undercurl),
            [4, 4] => Some(DottedUnderline),
            [4, 5] => Some(DashedUnderline),
            [4, ..] => Some(Underline),
            [5] => Some(BlinkSlow),
            [6] => Some(BlinkFast),
            [7] => Some(Reverse),
            [8] => Some(Hidden),
            [9] => Some(Strike),
            [21] => Some(CancelBold),
            [22] => Some(CancelBoldDim),
            [23] => Some(CancelItalic),
            [24] => Some(CancelUnderline),
            [25] => Some(CancelBlink),
            [27] => Some(CancelReverse),
            [28] => Some(CancelHidden),
            [29] => Some(CancelStrike),
            [30] => Some(Foreground(Color::Named(NamedColor::Black))),
            [31] => Some(Foreground(Color::Named(NamedColor::Red))),
            [32] => Some(Foreground(Color::Named(NamedColor::Green))),
            [33] => Some(Foreground(Color::Named(NamedColor::Yellow))),
            [34] => Some(Foreground(Color::Named(NamedColor::Blue))),
            [35] => Some(Foreground(Color::Named(NamedColor::Magenta))),
            [36] => Some(Foreground(Color::Named(NamedColor::Cyan))),
            [37] => Some(Foreground(Color::Named(NamedColor::White))),
            [38] => {
                let mut iter = params.map(|param| param[0]);
                subparam_sgr_to_color(&mut iter).map(Foreground)
            }
            [38, params @ ..] => handle_colon_rgb(params).map(Foreground),
            [39] => Some(Foreground(Color::Named(NamedColor::Foreground))),
            [40] => Some(Background(Color::Named(NamedColor::Black))),
            [41] => Some(Background(Color::Named(NamedColor::Red))),
            [42] => Some(Background(Color::Named(NamedColor::Green))),
            [43] => Some(Background(Color::Named(NamedColor::Yellow))),
            [44] => Some(Background(Color::Named(NamedColor::Blue))),
            [45] => Some(Background(Color::Named(NamedColor::Magenta))),
            [46] => Some(Background(Color::Named(NamedColor::Cyan))),
            [47] => Some(Background(Color::Named(NamedColor::White))),
            [48] => {
                let mut iter = params.map(|param| param[0]);
                subparam_sgr_to_color(&mut iter).map(Background)
            }
            [48, params @ ..] => handle_colon_rgb(params).map(Background),
            [49] => Some(Background(Color::Named(NamedColor::Background))),
            [58] => {
                let mut iter = params.map(|param| param[0]);
                subparam_sgr_to_color(&mut iter).map(|color| UnderlineColor(Some(color)))
            }
            [58, params @ ..] => handle_colon_rgb(params).map(|color| UnderlineColor(Some(color))),
            [59] => Some(UnderlineColor(None)),
            [90] => Some(Foreground(Color::Named(NamedColor::BrightBlack))),
            [91] => Some(Foreground(Color::Named(NamedColor::BrightRed))),
            [92] => Some(Foreground(Color::Named(NamedColor::BrightGreen))),
            [93] => Some(Foreground(Color::Named(NamedColor::BrightYellow))),
            [94] => Some(Foreground(Color::Named(NamedColor::BrightBlue))),
            [95] => Some(Foreground(Color::Named(NamedColor::BrightMagenta))),
            [96] => Some(Foreground(Color::Named(NamedColor::BrightCyan))),
            [97] => Some(Foreground(Color::Named(NamedColor::BrightWhite))),
            [100] => Some(Background(Color::Named(NamedColor::BrightBlack))),
            [101] => Some(Background(Color::Named(NamedColor::BrightRed))),
            [102] => Some(Background(Color::Named(NamedColor::BrightGreen))),
            [103] => Some(Background(Color::Named(NamedColor::BrightYellow))),
            [104] => Some(Background(Color::Named(NamedColor::BrightBlue))),
            [105] => Some(Background(Color::Named(NamedColor::BrightMagenta))),
            [106] => Some(Background(Color::Named(NamedColor::BrightCyan))),
            [107] => Some(Background(Color::Named(NamedColor::BrightWhite))),
            _ => None,
        };

        if let Some(attribute) = attribute {
            attributes.push(attribute);
        }
    }

    attributes
}
/// Handle colon separated rgb color escape sequence.
fn handle_colon_rgb(params: &[u16]) -> Option<Color> {
    let rgb_start = if params.len() > 4 { 2 } else { 1 };
    let rgb_iter = params[rgb_start..].iter().copied();
    let mut iter = iter::once(params[0]).chain(rgb_iter);

    subparam_sgr_to_color(&mut iter)
}

/// Parse a color specifier from list of attributes.
fn subparam_sgr_to_color<I>(params: &mut I) -> Option<Color>
where
    I: Iterator<Item = Subparam>,
{
    match params.next() {
        Some(2) => Some(Color::Spec(Rgb {
            r: u8::try_from(params.next()?).ok()?,
            g: u8::try_from(params.next()?).ok()?,
            b: u8::try_from(params.next()?).ok()?,
        })),
        Some(5) => Some(Color::Indexed(u8::try_from(params.next()?).ok()?)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use crate::ansi::handler::{Charset, CharsetIndex, CursorShape};

    use super::*;

    #[derive(Debug, PartialEq, Eq)]
    enum Event {
        SetTitle(String),
        SetCursorShape(CursorShape),
        SetCursorPosition(Position),
        SetCursorLine(Line),
        SetClipboard {
            clipboard: u8,
            payload: Vec<u8>,
        },
        SetCursorColumn(Column),
        SetCharsetIndex {
            index: CharsetIndex,
            charset: Charset,
        },
        SetActiveCharset(CharsetIndex),
        SetMode(Mode),
        SetPrivateMode(PrivateMode),
        SetAttribute(Attribute),
        MoveCursor {
            direction: Direction,
            count: usize,
            reset_column: bool,
        },
        PutChar(char),
        PutTab,
        PutHyperlink(Hyperlink),
        PutBlank(usize),
        ReportClipboard(u8),
        ReportTerminal,
        ReportMode(Mode),
        ClearScreen(ScreenClearMode),
        ClearLine(LineClearMode),
        SaveCursorPosition,
        RestoreCursorPosition,
        DeleteLines(usize),
        DeleteChars(usize),
        EraseChars(usize),
        Newline,
        CarriageReturn,
        RingBell,
        Backspace,
        Linefeed,
        Substitute,
        ResetState,
    }

    #[derive(Default)]
    struct MockHandler {
        events: Vec<Event>,
    }

    impl Handler for MockHandler {
        fn set_title(&mut self, title: &str) {
            self.events.push(Event::SetTitle(title.to_string()));
        }

        fn set_cursor_shape(&mut self, shape: CursorShape) {
            self.events.push(Event::SetCursorShape(shape));
        }

        fn set_cursor_position(&mut self, position: Position) {
            self.events.push(Event::SetCursorPosition(position))
        }

        fn set_cursor_line(&mut self, line: Line) {
            self.events.push(Event::SetCursorLine(line))
        }

        fn set_cursor_column(&mut self, column: Column) {
            self.events.push(Event::SetCursorColumn(column))
        }

        fn set_clipboard(&mut self, clipboard: u8, payload: &[u8]) {
            self.events.push(Event::SetClipboard {
                clipboard,
                payload: payload.to_vec(),
            });
        }

        fn set_mode(&mut self, mode: Mode) {
            self.events.push(Event::SetMode(mode));
        }

        fn set_private_mode(&mut self, mode: PrivateMode) {
            self.events.push(Event::SetPrivateMode(mode));
        }

        fn set_attribute(&mut self, attribute: Attribute) {
            self.events.push(Event::SetAttribute(attribute));
        }

        fn reset_state(&mut self) {
            self.events.push(Event::ResetState);
        }

        fn move_cursor(&mut self, direction: Direction, count: usize, reset_column: bool) {
            self.events.push(Event::MoveCursor {
                direction,
                count,
                reset_column,
            });
        }

        fn put_char(&mut self, c: char) {
            self.events.push(Event::PutChar(c));
        }

        fn put_tab(&mut self) {
            self.events.push(Event::PutTab);
        }

        fn put_hyperlink(&mut self, hyperlink: Hyperlink) {
            self.events.push(Event::PutHyperlink(hyperlink));
        }

        fn put_blank(&mut self, count: usize) {
            self.events.push(Event::PutBlank(count));
        }

        fn report_clipboard(&mut self, clipboard: u8) {
            self.events.push(Event::ReportClipboard(clipboard));
        }

        fn report_terminal(&mut self) {
            self.events.push(Event::ReportTerminal);
        }

        fn report_mode(&mut self, mode: Mode) {
            self.events.push(Event::ReportMode(mode));
        }

        fn clear_screen(&mut self, mode: ScreenClearMode) {
            self.events.push(Event::ClearScreen(mode));
        }

        fn clear_line(&mut self, mode: LineClearMode) {
            self.events.push(Event::ClearLine(mode));
        }

        fn save_cursor_position(&mut self) {
            self.events.push(Event::SaveCursorPosition);
        }

        fn restore_cursor_position(&mut self) {
            self.events.push(Event::RestoreCursorPosition);
        }

        fn delete_lines(&mut self, count: usize) {
            self.events.push(Event::DeleteLines(count));
        }

        fn delete_chars(&mut self, count: usize) {
            self.events.push(Event::DeleteChars(count));
        }

        fn erase_chars(&mut self, count: usize) {
            self.events.push(Event::EraseChars(count));
        }

        fn newline(&mut self) {
            self.events.push(Event::Newline);
        }

        fn carriage_return(&mut self) {
            self.events.push(Event::CarriageReturn);
        }

        fn ring_bell(&mut self) {
            self.events.push(Event::RingBell);
        }

        fn backspace(&mut self) {
            self.events.push(Event::Backspace);
        }

        fn linefeed(&mut self) {
            self.events.push(Event::Linefeed);
        }

        fn substitute(&mut self) {
            self.events.push(Event::Substitute);
        }

        fn set_charset_index(&mut self, index: CharsetIndex, charset: Charset) {
            self.events.push(Event::SetCharsetIndex { index, charset });
        }

        fn set_active_charset(&mut self, index: CharsetIndex) {
            self.events.push(Event::SetActiveCharset(index));
        }
    }

    #[test]
    fn set_title() {
        static BYTES: &[u8] = b"\x1b]0;title ; with semicolon\x07";

        let mut processor = Processor::new();
        let mut handler = MockHandler::default();

        processor.advance(&mut handler, BYTES);

        assert_eq!(
            handler.events,
            [Event::SetTitle("title ; with semicolon".to_string())]
        )
    }

    #[test]
    fn alternate_screen() {
        static BYTES: &[u8] = b"\x1b[?1049h";

        let mut processor = Processor::new();
        let mut handler = MockHandler::default();

        processor.advance(&mut handler, BYTES);

        assert_eq!(
            handler.events,
            [Event::SetPrivateMode(PrivateMode::Named(
                NamedPrivateMode::SwapScreenAndSetRestoreCursor
            ))]
        )
    }

    #[test]
    fn parse_fish_prompt() {
        static BYTES: &[u8] = b"\x1b[92mx\x1b(B\x1b[m@\x1b(B\x1b[mLains-MacBook\x1b(B\x1b[m \x1b[32m~/P/P/saiga\x1b(B\x1b[m (main)\x1b(B\x1b[m>";

        let mut handler = MockHandler::default();
        let mut processor = Processor::new();

        processor.advance(&mut handler, BYTES);

        assert_eq!(
            handler.events,
            [
                Event::SetAttribute(Attribute::Foreground(Color::Named(NamedColor::BrightGreen))),
                Event::PutChar('x'),
                Event::SetCharsetIndex {
                    index: CharsetIndex::G0,
                    charset: Charset::Ascii
                },
                Event::SetAttribute(Attribute::Reset),
                Event::PutChar('@'),
                Event::SetCharsetIndex {
                    index: CharsetIndex::G0,
                    charset: Charset::Ascii
                },
                Event::SetAttribute(Attribute::Reset),
                Event::PutChar('L'),
                Event::PutChar('a'),
                Event::PutChar('i'),
                Event::PutChar('n'),
                Event::PutChar('s'),
                Event::PutChar('-'),
                Event::PutChar('M'),
                Event::PutChar('a'),
                Event::PutChar('c'),
                Event::PutChar('B'),
                Event::PutChar('o'),
                Event::PutChar('o'),
                Event::PutChar('k'),
                Event::SetCharsetIndex {
                    index: CharsetIndex::G0,
                    charset: Charset::Ascii
                },
                Event::SetAttribute(Attribute::Reset),
                Event::PutChar(' '),
                Event::SetAttribute(Attribute::Foreground(Color::Named(NamedColor::Green))),
                Event::PutChar('~'),
                Event::PutChar('/'),
                Event::PutChar('P'),
                Event::PutChar('/'),
                Event::PutChar('P'),
                Event::PutChar('/'),
                Event::PutChar('s'),
                Event::PutChar('a'),
                Event::PutChar('i'),
                Event::PutChar('g'),
                Event::PutChar('a'),
                Event::SetCharsetIndex {
                    index: CharsetIndex::G0,
                    charset: Charset::Ascii
                },
                Event::SetAttribute(Attribute::Reset),
                Event::PutChar(' '),
                Event::PutChar('('),
                Event::PutChar('m'),
                Event::PutChar('a'),
                Event::PutChar('i'),
                Event::PutChar('n'),
                Event::PutChar(')'),
                Event::SetCharsetIndex {
                    index: CharsetIndex::G0,
                    charset: Charset::Ascii
                },
                Event::SetAttribute(Attribute::Reset),
                Event::PutChar('>')
            ],
        )
    }

    #[test]
    fn parse_control_attribute() {
        static BYTES: &[u8] = &[0x1b, b'[', b'1', b'm'];

        let mut processor = Processor::new();
        let mut handler = MockHandler::default();

        processor.advance(&mut handler, BYTES);

        assert_eq!(handler.events, [Event::SetAttribute(Attribute::Bold)]);
    }

    #[test]
    fn parse_truecolor_attr() {
        static BYTES: &[u8] = &[
            0x1b, b'[', b'3', b'8', b';', b'2', b';', b'1', b'2', b'8', b';', b'6', b'6', b';',
            b'2', b'5', b'5', b'm',
        ];

        let mut processor = Processor::new();
        let mut handler = MockHandler::default();

        processor.advance(&mut handler, BYTES);

        let rgb = Rgb {
            r: 128,
            g: 66,
            b: 255,
        };

        assert_eq!(
            handler.events,
            [Event::SetAttribute(Attribute::Foreground(Color::Spec(rgb)))]
        )
    }

    #[test]
    fn parse_designate_g0_as_line_drawing() {
        static BYTES: &[u8] = &[0x1b, b'(', b'0'];

        let mut processor = Processor::new();
        let mut handler = MockHandler::default();

        processor.advance(&mut handler, BYTES);

        assert_eq!(
            handler.events,
            [Event::SetCharsetIndex {
                index: CharsetIndex::G0,
                charset: Charset::SpecialCharacterAndLineDrawing
            }]
        )
    }

    #[test]
    fn parse_designate_g1_as_line_drawing_and_invoke() {
        static BYTES: &[u8] = &[0x1b, b')', b'0', 0x0e];

        let mut processor = Processor::new();
        let mut handler = MockHandler::default();

        processor.advance(&mut handler, BYTES);

        assert_eq!(
            handler.events,
            [
                Event::SetCharsetIndex {
                    index: CharsetIndex::G1,
                    charset: Charset::SpecialCharacterAndLineDrawing
                },
                Event::SetActiveCharset(CharsetIndex::G1)
            ]
        );
    }
}
