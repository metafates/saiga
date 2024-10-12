use log::debug;

use crate::{Executor, MAX_INTERMEDIATES, param, utf8};
use crate::ansi::handler::{Hyperlink, LineClearMode, ScreenClearMode};
use super::{c0, handler::{Column, Direction, Handler, Line, Position}};

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
            c0::SI => self.handler.set_charset(super::handler::CharsetIndex::G0),
            c0::SO => self.handler.set_charset(super::handler::CharsetIndex::G1),
            c0::SUB => self.handler.substitute(),
            _ => debug!("[unhandled] execute byte={byte:02x}"),
        }
    }

    fn put(&mut self, byte: u8) {
        debug!("[unhandled] put byte={byte:02x}")
    }

    fn hook(
        &mut self,
        params: &crate::param::Params,
        intermediates: &[u8],
        ignore: bool,
        action: char,
    ) {
        debug!("[unhandled] hook")
    }

    fn unhook(&mut self) {
        debug!("[unhandled] unhook")
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], bell_terminated: bool) {
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
            [b"8", params, uri]
                if !uri.is_empty() && URI_PREFIXES.into_iter().any(|p| uri.starts_with(p)) =>
            {
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
                    b"?" => self.handler.write_clipboard(*clipboard),
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
                debug!("unhandled")
            }
        }
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], _ignore: bool, byte: u8) {
        match (byte, intermediates) {
            (b'D', []) => self.handler.linefeed(),
            (b'E', []) => {
                self.handler.linefeed();
                self.handler.carriage_return();
            }
            (b'Z', []) => self.handler.write_terminal(),
            (b'7', []) => self.handler.save_cursor_position(),
            (b'8', []) => self.handler.restore_cursor_position(),

            // String terminator, do nothing (parser handles as string terminator).
            (b'\\', []) => (),

            _ => debug!("unhandled"),
        }
    }

    fn csi_dispatch(
        &mut self,
        params: &crate::param::Params,
        intermediates: &[u8],
        ignore: bool,
        action: char,
    ) {
        if ignore || intermediates.len() > MAX_INTERMEDIATES {
            debug!("unhandled");
            return;
        }

        let mut params = params.into_iter();
        let mut next_param_or = |default: u16| match params.next() {
            Some(param) => match param.into_iter().next() {
                Some(subparam) if subparam != 0 => subparam,
                _ => default,
            },
            _ => default,
        };

        match (action, intermediates) {
            ('@', []) => self.handler.put_blank(next_param_or(1) as usize),
            ('A', []) => self
                .handler
                .move_cursor(Direction::Up, next_param_or(1) as usize, false),
            ('B' | 'e', []) => {
                self.handler
                    .move_cursor(Direction::Down, next_param_or(1) as usize, false)
            }
            ('C' | 'a', []) => {
                self.handler
                    .move_cursor(Direction::Right, next_param_or(1) as usize, false)
            }
            ('c', _) => self.handler.write_terminal(), // TODO: pass intermediates?
            ('D', []) => {
                self.handler
                    .move_cursor(Direction::Left, next_param_or(1) as usize, false)
            }
            ('d', []) => self.handler.set_cursor_line(next_param_or(1) as Line - 1),
            ('E', []) => self
                .handler
                .move_cursor(Direction::Down, next_param_or(1) as usize, true),
            ('F', []) => self
                .handler
                .move_cursor(Direction::Up, next_param_or(1) as usize, true),
            ('G' | '`', []) => self
                .handler
                .set_cursor_column(next_param_or(1) as Column - 1),
            ('H' | 'f', []) => {
                let line = next_param_or(1) as Line;
                let column = next_param_or(1) as Column;

                self.handler.set_cursor_position(Position { line, column });
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
            // TODO: rest
            _ => debug!("unhandled"),
        }
    }
}
