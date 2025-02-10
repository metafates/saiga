use std::{
    fmt::Write,
    iter,
    time::{Duration, Instant},
};

use log::debug;

use super::{
    c0,
    handler::{Charset, CharsetIndex, Handler, Rgb},
};
use crate::{
    ansi::handler::{
        Attribute, Color, CursorShape, CursorStyle, Hyperlink, KeyboardModes,
        KeyboardModesApplyBehavior, LineClearMode, Mode, ModifyOtherKeys, NamedColor,
        NamedPrivateMode, PrivateMode, ScpCharPath, ScpUpdateMode, ScreenClearMode,
        TabulationClearMode,
    },
    param::{Param, Subparam},
};
use crate::{param, Executor, MAX_INTERMEDIATES};

/// Maximum time before a synchronized update is aborted.
const SYNC_UPDATE_TIMEOUT: Duration = Duration::from_millis(150);

/// Maximum number of bytes read in one synchronized update (2MiB).
const SYNC_BUFFER_SIZE: usize = 0x20_0000;

/// Number of bytes in the BSU/ESU CSI sequences.
const SYNC_ESCAPE_LEN: usize = 8;

/// BSU CSI sequence for beginning or extending synchronized updates.
const BSU_CSI: [u8; SYNC_ESCAPE_LEN] = *b"\x1b[?2026h";

/// ESU CSI sequence for terminating synchronized updates.
const ESU_CSI: [u8; SYNC_ESCAPE_LEN] = *b"\x1b[?2026l";

/// Interface for creating timeouts and checking their expiry.
///
/// This is internally used by the [`Processor`] to handle synchronized
/// updates.
pub trait Timeout: Default {
    /// Sets the timeout for the next synchronized update.
    ///
    /// The `duration` parameter specifies the duration of the timeout. Once the
    /// specified duration has elapsed, the synchronized update rotuine can be
    /// performed.
    fn set_timeout(&mut self, duration: Duration);
    /// Clear the current timeout.
    fn clear_timeout(&mut self);
    /// Returns whether a timeout is currently active and has not yet expired.
    fn pending_timeout(&self) -> bool;
}

#[derive(Default, Debug)]
pub struct StdSyncHandler {
    timeout: Option<Instant>,
}

impl StdSyncHandler {
    /// Synchronized update expiration time.
    #[inline]
    pub fn sync_timeout(&self) -> Option<Instant> {
        self.timeout
    }
}

impl StdSyncHandler {
    #[inline]
    fn set_timeout(&mut self, duration: Duration) {
        self.timeout = Some(Instant::now() + duration);
    }

    #[inline]
    fn clear_timeout(&mut self) {
        self.timeout = None;
    }

    #[inline]
    fn pending_timeout(&self) -> bool {
        self.timeout.is_some()
    }
}

/// Internal state for VTE processor.
#[derive(Debug, Default)]
struct ProcessorState {
    /// Last processed character for repetition.
    preceding_char: Option<char>,

    /// State for synchronized terminal updates.
    sync_state: SyncState,
}

#[derive(Debug)]
struct SyncState {
    /// Handler for synchronized updates.
    timeout: StdSyncHandler,

    /// Bytes read during the synchronized update.
    buffer: Vec<u8>,
}

impl Default for SyncState {
    fn default() -> Self {
        Self {
            buffer: Vec::with_capacity(SYNC_BUFFER_SIZE),
            timeout: StdSyncHandler::default(),
        }
    }
}

#[derive(Default)]
pub struct Processor {
    state: ProcessorState,
    parser: crate::Parser,
}

impl Processor {
    pub fn new() -> Self {
        Default::default()
    }

    /// Synchronized update timeout.
    pub fn sync_timeout(&self) -> &StdSyncHandler {
        &self.state.sync_state.timeout
    }

    pub fn advance<H: Handler>(&mut self, handler: &mut H, bytes: &[u8]) {
        if self.state.sync_state.timeout.pending_timeout() {
            self.advance_sync(handler, bytes);
        } else {
            let mut executor = HandlerExecutor::new(&mut self.state, handler);

            self.parser.advance(&mut executor, bytes);
        }
    }

    /// End a synchronized update.
    pub fn stop_sync<H>(&mut self, handler: &mut H)
    where
        H: Handler,
    {
        // Process all synchronized bytes.
        let bytes = self.state.sync_state.buffer.clone(); // TODO: avoid clone?
        let mut performer = HandlerExecutor::new(&mut self.state, handler);
        self.parser.advance(&mut performer, bytes.as_slice());

        // Report that update ended, since we could end due to timeout.
        handler.unset_private_mode(NamedPrivateMode::SyncUpdate.into());
        // Resetting state after processing makes sure we don't interpret buffered sync escapes.
        self.state.sync_state.buffer.clear();
        self.state.sync_state.timeout.clear_timeout();
    }

    /// Number of bytes in the synchronization buffer.
    #[inline]
    pub fn sync_bytes_count(&self) -> usize {
        self.state.sync_state.buffer.len()
    }

    /// Process a new byte during a synchronized update.
    #[cold]
    fn advance_sync<H>(&mut self, handler: &mut H, bytes: &[u8])
    where
        H: Handler,
    {
        self.state.sync_state.buffer.extend_from_slice(bytes);

        // Handle sync CSI escape sequences.
        self.advance_sync_csi(handler);
    }

    /// Handle BSU/ESU CSI sequences during synchronized update.
    fn advance_sync_csi<H>(&mut self, handler: &mut H)
    where
        H: Handler,
    {
        // Get the last few bytes for comparison.
        let len = self.state.sync_state.buffer.len();
        let offset = len.saturating_sub(SYNC_ESCAPE_LEN);
        let end = &self.state.sync_state.buffer[offset..];

        if end == BSU_CSI {
            self.state
                .sync_state
                .timeout
                .set_timeout(SYNC_UPDATE_TIMEOUT);
        } else if end == ESU_CSI || len >= SYNC_BUFFER_SIZE - 1 {
            self.stop_sync(handler);
        }
    }
}

struct HandlerExecutor<'a, H: Handler> {
    state: &'a mut ProcessorState,
    handler: &'a mut H,
}

impl<'a, H: Handler + 'a> HandlerExecutor<'a, H> {
    fn new<'b>(state: &'b mut ProcessorState, handler: &'b mut H) -> HandlerExecutor<'b, H> {
        HandlerExecutor { state, handler }
    }
}

impl<H: Handler> Executor for HandlerExecutor<'_, H> {
    fn print(&mut self, c: char) {
        self.handler.input(c);
        self.state.preceding_char = Some(c)
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            c0::HT => self.handler.put_tab(1),
            c0::CR => self.handler.carriage_return(),
            c0::BS => self.handler.backspace(),
            c0::BEL => self.handler.bell(),
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
        if params.is_empty() || params[0].is_empty() {
            return;
        }

        let terminator = if bell_terminated { "\x07" } else { "\x1b\\" };

        macro_rules! unhandled {
            () => {{
                let mut buf = String::new();
                for items in params {
                    buf.push('[');
                    for item in *items {
                        let _ = write!(buf, "{:?}", *item as char);
                    }
                    buf.push_str("],");
                }

                debug!("[unhandled osc_dispatch]: [{}] at line {}", &buf, line!());
            }};
        }

        match params {
            // set window title
            [b"0" | b"2", title @ ..] => {
                let title = title
                    .iter()
                    .flat_map(|x| simdutf8::basic::from_utf8(x))
                    .collect::<Vec<&str>>()
                    .join(&param::PARAM_SEPARATOR.to_string())
                    .trim()
                    .to_string();

                self.handler.set_title(Some(title.to_string()));
            }

            // Change color number
            [b"4", params @ ..] if !params.is_empty() && params.len() % 2 == 0 => {
                for chunk in params.chunks(2) {
                    let index = match parse_number(chunk[0]) {
                        Some(index) => index,
                        None => {
                            unhandled!();
                            continue;
                        }
                    };

                    if let Some(c) = xparse_color(chunk[1]) {
                        self.handler.set_color(index as usize, c);
                    } else if chunk[1] == b"?" {
                        let prefix = format!("4;{index}");
                        self.handler
                            .dynamic_color_sequence(prefix, index as usize, terminator);
                    } else {
                        unhandled!();
                    }
                }
            }

            // Create a hyperlink to uri using params.
            [b"8", link_params, uri, rest @ ..] => {
                let mut uri = simdutf8::basic::from_utf8(uri)
                    .unwrap_or_default()
                    .to_string();

                for param in rest {
                    uri.push(param::PARAM_SEPARATOR.into());
                    uri.push_str(simdutf8::basic::from_utf8(param).unwrap_or_default());
                }

                if uri.is_empty() {
                    self.handler.set_hyperlink(None);
                }

                let id = link_params
                    .split(|&b| b == b':')
                    .find_map(|kv| kv.strip_prefix(b"id="))
                    .and_then(|kv| simdutf8::basic::from_utf8(kv).ok().map(|e| e.to_owned()));

                self.handler.set_hyperlink(Some(Hyperlink { id, uri }));
            }

            [color_num @ (b"10" | b"11" | b"12"), params @ ..] if !params.is_empty() => {
                let Some(mut dynamic_code) = parse_number(color_num) else {
                    return;
                };

                for param in params {
                    // 10 is the first dynamic color, also the foreground.
                    let offset = dynamic_code as usize - 10;
                    let index = NamedColor::Foreground as usize + offset;

                    // End of setting dynamic colors.
                    if index > NamedColor::Cursor as usize {
                        unhandled!();
                        break;
                    }

                    if let Some(color) = xparse_color(param) {
                        self.handler.set_color(index, color);
                    } else if param == b"?" {
                        self.handler.dynamic_color_sequence(
                            dynamic_code.to_string(),
                            index,
                            terminator,
                        );
                    } else {
                        unhandled!();
                    }

                    dynamic_code += 1;
                }
            }

            [b"50", param] if param.len() >= 13 && param[0..12] == *b"CursorShape=" => {
                let shape = match param[12] as char {
                    '0' => CursorShape::Block,
                    '1' => CursorShape::Beam,
                    '2' => CursorShape::Underline,
                    _ => {
                        unhandled!();
                        return;
                    }
                };

                self.handler.set_cursor_shape(shape);
            }

            // Set or query clipboard
            [b"52", clipboard, payload, ..] => {
                let clipboard = clipboard.first().unwrap_or(&b'c');

                match *payload {
                    b"?" => self.handler.clipboard_load(*clipboard, terminator),
                    base64 => self.handler.clipboard_store(*clipboard, base64),
                }
            }

            // Reset color number `color` to themed color.
            [b"104", indices @ ..] => {
                // Reset all color indexes when no parameters are given.
                if indices.is_empty() || indices[0].is_empty() {
                    for i in 0..256 {
                        self.handler.reset_color(i);
                    }

                    return;
                }

                // Reset color indexes given as parameters.
                for param in indices {
                    match parse_number(param) {
                        Some(index) => self.handler.reset_color(index as usize),
                        None => unhandled!(),
                    }
                }
            }

            // Restore default foreground to themed color.
            [b"110"] => self.handler.reset_color(NamedColor::Foreground as usize),

            // Restore default background to themed color.
            [b"111"] => self.handler.reset_color(NamedColor::Background as usize),

            // Restore default cursor to themed color.
            [b"112"] => self.handler.reset_color(NamedColor::Cursor as usize),

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
            (b'0', [index, ..]) => {
                let Ok(index) = CharsetIndex::try_from(*index) else {
                    unhandled!();
                    return;
                };

                self.handler
                    .configure_charset(index, Charset::SpecialCharacterAndLineDrawing);
            }
            (b'B', [index, ..]) => {
                let Ok(index) = CharsetIndex::try_from(*index) else {
                    unhandled!();
                    return;
                };

                self.handler.configure_charset(index, Charset::Ascii);
            }
            (b'D', []) => self.handler.linefeed(),
            (b'E', []) => {
                self.handler.linefeed();
                self.handler.carriage_return();
            }
            (b'H', []) => self.handler.set_horizontal_tabstop(),
            (b'M', []) => self.handler.reverse_index(),
            (b'Z', []) => self.handler.identify_terminal(None),
            (b'c', []) => self.handler.reset_state(),
            (b'7', []) => self.handler.save_cursor_position(),
            (b'8', [b'#']) => self.handler.decaln(),
            (b'8', []) => self.handler.restore_cursor_position(),
            (b'=', []) => self.handler.set_keypad_application_mode(),
            (b'>', []) => self.handler.unset_keypad_application_mode(),

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

        let mut params_iter = params.as_slice().iter();
        let handler = &mut self.handler;

        let mut next_param_or = |default: Subparam| match params_iter.next().map(Param::as_slice) {
            Some(&[subparam, ..]) if subparam != 0 => subparam,
            _ => default,
        };

        match (action, intermediates) {
            ('@', []) => handler.insert_blank(next_param_or(1).into()),
            ('A', []) => handler.move_up(next_param_or(1).into()),
            ('B' | 'e', []) => handler.move_down(next_param_or(1).into()),
            ('b', []) => {
                if let Some(c) = self.state.preceding_char {
                    for _ in 0..next_param_or(1) {
                        handler.input(c);
                    }
                } else {
                    debug!("tried to repeat with no preceding char");
                }
            }
            ('C' | 'a', []) => handler.move_forward(next_param_or(1).into()),
            ('c', intermediates) if next_param_or(0) == 0 => {
                handler.identify_terminal(intermediates.first().map(|&i| i as char))
            }
            ('D', []) => handler.move_backward(next_param_or(1) as usize),
            ('d', []) => handler.goto_line(next_param_or(1) as i32 - 1),
            ('E', []) => handler.move_down_and_cr(next_param_or(1) as usize),
            ('F', []) => handler.move_up_and_cr(next_param_or(1) as usize),
            ('G' | '`', []) => handler.goto_col(next_param_or(1) as usize - 1),
            ('W', [b'?']) if next_param_or(0) == 5 => handler.put_tab(8),
            ('g', []) => {
                let mode = match next_param_or(0) {
                    0 => TabulationClearMode::Current,
                    3 => TabulationClearMode::All,
                    _ => {
                        unhandled!();
                        return;
                    }
                };

                handler.clear_tabs(mode);
            }
            ('H' | 'f', []) => {
                let y = next_param_or(1) as i32;
                let x = next_param_or(1) as usize;

                handler.goto(y - 1, x - 1);
            }
            ('h', []) => {
                for param in params_iter.map(|param| param[0]) {
                    handler.set_mode(Mode::new(param));
                }
            }
            ('h', [b'?']) => {
                for param in params_iter.map(|p| p.as_slice()[0]) {
                    if param == NamedPrivateMode::SyncUpdate as u16 {
                        self.state
                            .sync_state
                            .timeout
                            .set_timeout(SYNC_UPDATE_TIMEOUT);
                    }

                    handler.set_private_mode(PrivateMode::new(param));
                }
            }
            ('I', []) => handler.move_forward_tabs(next_param_or(1)),
            ('J', []) => {
                let mode = match next_param_or(0) {
                    0 => ScreenClearMode::Below,
                    1 => ScreenClearMode::Above,
                    2 => ScreenClearMode::All,
                    3 => ScreenClearMode::Saved,
                    _ => {
                        unhandled!();
                        return;
                    }
                };

                self.handler.clear_screen(mode);
            }
            ('K', []) => {
                let mode = match next_param_or(0) {
                    0 => LineClearMode::Right,
                    1 => LineClearMode::Left,
                    2 => LineClearMode::All,
                    _ => {
                        unhandled!();
                        return;
                    }
                };

                self.handler.clear_line(mode);
            }
            ('k', [b' ']) => {
                // SCP control.
                let char_path = match next_param_or(0) {
                    0 => ScpCharPath::Default,
                    1 => ScpCharPath::LTR,
                    2 => ScpCharPath::RTL,
                    _ => {
                        unhandled!();
                        return;
                    }
                };

                let update_mode = match next_param_or(0) {
                    0 => ScpUpdateMode::ImplementationDependant,
                    1 => ScpUpdateMode::DataToPresentation,
                    2 => ScpUpdateMode::PresentationToData,
                    _ => {
                        unhandled!();
                        return;
                    }
                };

                handler.set_scp(char_path, update_mode);
            }
            ('L', []) => handler.insert_blank_lines(next_param_or(1) as usize),
            ('l', []) => {
                for param in params_iter.map(|param| param[0]) {
                    handler.unset_mode(Mode::new(param));
                }
            }
            ('l', [b'?']) => {
                for param in params_iter.map(|param| param[0]) {
                    handler.unset_private_mode(PrivateMode::new(param));
                }
            }
            ('M', []) => handler.delete_lines(next_param_or(1) as usize),
            ('m', []) => {
                if params.is_empty() {
                    handler.terminal_attribute(Attribute::Reset);
                    return;
                }

                attrs_from_sgr_parameters(*handler, &mut params_iter);
            }
            ('m', [b'>']) => {
                let mode = match (next_param_or(1) == 4).then(|| next_param_or(0)) {
                    Some(0) => ModifyOtherKeys::Reset,
                    Some(1) => ModifyOtherKeys::EnableExceptWellDefined,
                    Some(2) => ModifyOtherKeys::EnableAll,
                    _ => {
                        unhandled!();
                        return;
                    }
                };

                handler.set_modify_other_keys(mode);
            }
            ('m', [b'?']) => {
                let Some(next) = params_iter.next() else {
                    unhandled!();
                    return;
                };

                if next.as_slice() == [4] {
                    handler.report_modify_other_keys();
                }
            }
            ('n', []) => handler.device_status(next_param_or(0) as usize),
            ('P', []) => handler.delete_chars(next_param_or(1).into()),
            ('p', [b'$']) => {
                let mode = Mode::new(next_param_or(0));

                handler.report_mode(mode)
            }
            ('p', [b'?', b'$']) => {
                let mode = PrivateMode::new(next_param_or(0));

                handler.report_private_mode(mode)
            }
            ('q', [b' ']) => {
                let style_id = next_param_or(0);

                let shape = match style_id {
                    0 => None,
                    1 | 2 => Some(CursorShape::Block),
                    3 | 4 => Some(CursorShape::Underline),
                    5 | 6 => Some(CursorShape::Beam),
                    _ => {
                        unhandled!();
                        return;
                    }
                };

                let blinking = style_id % 2 == 1;
                let style = shape.map(|shape| CursorStyle { shape, blinking });

                handler.set_cursor_style(style);
            }
            ('r', []) => {
                let top = next_param_or(1) as usize;

                let bottom = params_iter
                    .next()
                    .map(|p| p[0] as usize)
                    .filter(|&p| p != 0);

                handler.set_scrolling_region(top, bottom);
            }
            ('S', []) => handler.scroll_up(next_param_or(1) as usize),
            ('s', []) => handler.save_cursor_position(),
            ('T', []) => handler.scroll_down(next_param_or(1) as usize),
            ('t', []) => match next_param_or(1) as usize {
                14 => handler.text_area_size_pixels(),
                18 => handler.text_area_size_chars(),
                22 => handler.push_title(),
                23 => handler.pop_title(),
                _ => unhandled!(),
            },
            ('u', [b'?']) => handler.report_keyboard_mode(),
            ('u', [b'=']) => {
                let mode = KeyboardModes::from_bits_truncate(next_param_or(0) as u8);
                let behavior = match next_param_or(1) {
                    3 => KeyboardModesApplyBehavior::Difference,
                    2 => KeyboardModesApplyBehavior::Union,
                    // Default is replace.
                    _ => KeyboardModesApplyBehavior::Replace,
                };
                handler.set_keyboard_mode(mode, behavior);
            }
            ('u', [b'>']) => {
                let mode = KeyboardModes::from_bits_truncate(next_param_or(0) as u8);

                handler.push_keyboard_mode(mode);
            }
            ('u', [b'<']) => {
                // The default is 1.
                handler.pop_keyboard_modes(next_param_or(1));
            }
            ('u', []) => handler.restore_cursor_position(),
            ('X', []) => handler.erase_chars(next_param_or(1).into()),
            ('Z', []) => handler.move_backward_tabs(next_param_or(1)),

            _ => unhandled!(),
        }
    }
}

#[inline]
fn attrs_from_sgr_parameters<'a>(
    handler: &mut impl Handler,
    params: &mut impl Iterator<Item = &'a Param>,
) {
    while let Some(param) = params.next() {
        let attr = match param.as_slice() {
            [0] => Some(Attribute::Reset),
            [1] => Some(Attribute::Bold),
            [2] => Some(Attribute::Dim),
            [3] => Some(Attribute::Italic),
            [4, 0] => Some(Attribute::CancelUnderline),
            [4, 2] => Some(Attribute::DoubleUnderline),
            [4, 3] => Some(Attribute::Undercurl),
            [4, 4] => Some(Attribute::DottedUnderline),
            [4, 5] => Some(Attribute::DashedUnderline),
            [4, ..] => Some(Attribute::Underline),
            [5] => Some(Attribute::BlinkSlow),
            [6] => Some(Attribute::BlinkFast),
            [7] => Some(Attribute::Reverse),
            [8] => Some(Attribute::Hidden),
            [9] => Some(Attribute::Strike),
            [21] => Some(Attribute::CancelBold),
            [22] => Some(Attribute::CancelBoldDim),
            [23] => Some(Attribute::CancelItalic),
            [24] => Some(Attribute::CancelUnderline),
            [25] => Some(Attribute::CancelBlink),
            [27] => Some(Attribute::CancelReverse),
            [28] => Some(Attribute::CancelHidden),
            [29] => Some(Attribute::CancelStrike),
            [30] => Some(Attribute::Foreground(Color::Named(NamedColor::Black))),
            [31] => Some(Attribute::Foreground(Color::Named(NamedColor::Red))),
            [32] => Some(Attribute::Foreground(Color::Named(NamedColor::Green))),
            [33] => Some(Attribute::Foreground(Color::Named(NamedColor::Yellow))),
            [34] => Some(Attribute::Foreground(Color::Named(NamedColor::Blue))),
            [35] => Some(Attribute::Foreground(Color::Named(NamedColor::Magenta))),
            [36] => Some(Attribute::Foreground(Color::Named(NamedColor::Cyan))),
            [37] => Some(Attribute::Foreground(Color::Named(NamedColor::White))),
            [38] => {
                let mut iter = params.map(|param| param[0]);
                parse_sgr_color(&mut iter).map(Attribute::Foreground)
            }
            [38, params @ ..] => handle_colon_rgb(params).map(Attribute::Foreground),
            [39] => Some(Attribute::Foreground(Color::Named(NamedColor::Foreground))),
            [40] => Some(Attribute::Background(Color::Named(NamedColor::Black))),
            [41] => Some(Attribute::Background(Color::Named(NamedColor::Red))),
            [42] => Some(Attribute::Background(Color::Named(NamedColor::Green))),
            [43] => Some(Attribute::Background(Color::Named(NamedColor::Yellow))),
            [44] => Some(Attribute::Background(Color::Named(NamedColor::Blue))),
            [45] => Some(Attribute::Background(Color::Named(NamedColor::Magenta))),
            [46] => Some(Attribute::Background(Color::Named(NamedColor::Cyan))),
            [47] => Some(Attribute::Background(Color::Named(NamedColor::White))),
            [48] => {
                let mut iter = params.map(|param| param[0]);
                parse_sgr_color(&mut iter).map(Attribute::Background)
            }
            [48, params @ ..] => handle_colon_rgb(params).map(Attribute::Background),
            [49] => Some(Attribute::Background(Color::Named(NamedColor::Background))),
            [58] => {
                let mut iter = params.map(|param| param[0]);
                parse_sgr_color(&mut iter).map(|color| Attribute::UnderlineColor(Some(color)))
            }
            [58, params @ ..] => {
                handle_colon_rgb(params).map(|color| Attribute::UnderlineColor(Some(color)))
            }
            [59] => Some(Attribute::UnderlineColor(None)),
            [90] => Some(Attribute::Foreground(Color::Named(NamedColor::BrightBlack))),
            [91] => Some(Attribute::Foreground(Color::Named(NamedColor::BrightRed))),
            [92] => Some(Attribute::Foreground(Color::Named(NamedColor::BrightGreen))),
            [93] => Some(Attribute::Foreground(Color::Named(
                NamedColor::BrightYellow,
            ))),
            [94] => Some(Attribute::Foreground(Color::Named(NamedColor::BrightBlue))),
            [95] => Some(Attribute::Foreground(Color::Named(
                NamedColor::BrightMagenta,
            ))),
            [96] => Some(Attribute::Foreground(Color::Named(NamedColor::BrightCyan))),
            [97] => Some(Attribute::Foreground(Color::Named(NamedColor::BrightWhite))),
            [100] => Some(Attribute::Background(Color::Named(NamedColor::BrightBlack))),
            [101] => Some(Attribute::Background(Color::Named(NamedColor::BrightRed))),
            [102] => Some(Attribute::Background(Color::Named(NamedColor::BrightGreen))),
            [103] => Some(Attribute::Background(Color::Named(
                NamedColor::BrightYellow,
            ))),
            [104] => Some(Attribute::Background(Color::Named(NamedColor::BrightBlue))),
            [105] => Some(Attribute::Background(Color::Named(
                NamedColor::BrightMagenta,
            ))),
            [106] => Some(Attribute::Background(Color::Named(NamedColor::BrightCyan))),
            [107] => Some(Attribute::Background(Color::Named(NamedColor::BrightWhite))),
            _ => None,
        };

        match attr {
            Some(attr) => handler.terminal_attribute(attr),
            None => continue,
        }
    }
}

/// Handle colon separated rgb color escape sequence.
fn handle_colon_rgb(params: &[u16]) -> Option<Color> {
    let rgb_start = if params.len() > 4 { 2 } else { 1 };
    let rgb_iter = params[rgb_start..].iter().copied();
    let mut iter = iter::once(params[0]).chain(rgb_iter);

    subparam_sgr_to_color(&mut iter)
}

/// Parse a color specifier from list of attributes.
fn subparam_sgr_to_color(params: &mut impl Iterator<Item = Subparam>) -> Option<Color> {
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

/// Parse colors in XParseColor format.
fn xparse_color(color: &[u8]) -> Option<Rgb> {
    if !color.is_empty() && color[0] == b'#' {
        parse_legacy_color(&color[1..])
    } else if color.len() >= 4 && &color[..4] == b"rgb:" {
        parse_rgb_color(&color[4..])
    } else {
        None
    }
}

/// Parse colors in `rgb:r(rrr)/g(ggg)/b(bbb)` format.
fn parse_rgb_color(color: &[u8]) -> Option<Rgb> {
    let colors = simdutf8::basic::from_utf8(color)
        .ok()?
        .split('/')
        .collect::<Vec<_>>();

    if colors.len() != 3 {
        return None;
    }

    // Scale values instead of filling with `0`s.
    let scale = |input: &str| {
        if input.len() > 4 {
            None
        } else {
            let max = u32::pow(16, input.len() as u32) - 1;
            let value = u32::from_str_radix(input, 16).ok()?;
            Some((255 * value / max) as u8)
        }
    };

    Some(Rgb {
        r: scale(colors[0])?,
        g: scale(colors[1])?,
        b: scale(colors[2])?,
    })
}

/// Parse colors in `#r(rrr)g(ggg)b(bbb)` format.
fn parse_legacy_color(color: &[u8]) -> Option<Rgb> {
    let item_len = color.len() / 3;

    // Truncate/Fill to two byte precision.
    let color_from_slice = |slice: &[u8]| {
        let col = usize::from_str_radix(simdutf8::basic::from_utf8(slice).ok()?, 16).ok()? << 4;
        Some((col >> (4 * slice.len().saturating_sub(1))) as u8)
    };

    Some(Rgb {
        r: color_from_slice(&color[0..item_len])?,
        g: color_from_slice(&color[item_len..item_len * 2])?,
        b: color_from_slice(&color[item_len * 2..])?,
    })
}

fn parse_number(input: &[u8]) -> Option<u8> {
    if input.is_empty() {
        return None;
    }
    let mut num: u8 = 0;
    for c in input {
        let c = *c as char;
        let digit = c.to_digit(10)?;
        num = num
            .checked_mul(10)
            .and_then(|v| v.checked_add(digit as u8))?;
    }
    Some(num)
}

/// Parse a color specifier from list of attributes.
fn parse_sgr_color(params: &mut impl Iterator<Item = u16>) -> Option<Color> {
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

// Tests for parsing escape sequences.
//
// Byte sequences used in these tests are recording of pty stdout.
#[cfg(test)]
mod tests {
    use super::*;

    struct MockHandler {
        index: CharsetIndex,
        charset: Charset,
        attr: Option<Attribute>,
        identity_reported: bool,
        color: Option<Rgb>,
        reset_colors: Vec<usize>,
        cursor_style: Option<CursorStyle>,
    }

    impl Handler for MockHandler {
        fn terminal_attribute(&mut self, attr: Attribute) {
            self.attr = Some(attr);
        }

        fn configure_charset(&mut self, index: CharsetIndex, charset: Charset) {
            self.index = index;
            self.charset = charset;
        }

        fn set_active_charset(&mut self, index: CharsetIndex) {
            self.index = index;
        }

        fn identify_terminal(&mut self, _intermediate: Option<char>) {
            self.identity_reported = true;
        }

        fn reset_state(&mut self) {
            *self = Self::default();
        }

        fn set_color(&mut self, _: usize, c: Rgb) {
            self.color = Some(c);
        }

        fn reset_color(&mut self, index: usize) {
            self.reset_colors.push(index)
        }

        fn set_cursor_style(&mut self, style: Option<CursorStyle>) {
            self.cursor_style = style;
        }
    }

    impl Default for MockHandler {
        fn default() -> MockHandler {
            MockHandler {
                index: CharsetIndex::G0,
                charset: Charset::Ascii,
                attr: None,
                identity_reported: false,
                color: None,
                reset_colors: Vec::new(),
                cursor_style: None,
            }
        }
    }

    #[test]
    fn parse_control_attribute() {
        static BYTES: &[u8] = &[0x1b, b'[', b'1', b'm'];

        let mut parser = Processor::new();
        let mut handler = MockHandler::default();

        parser.advance(&mut handler, BYTES);

        assert_eq!(handler.attr, Some(Attribute::Bold));
    }

    #[test]
    fn parse_terminal_identity_csi() {
        let bytes: &[u8] = &[0x1b, b'[', b'1', b'c'];

        let mut parser = Processor::new();
        let mut handler = MockHandler::default();

        parser.advance(&mut handler, bytes);

        assert!(!handler.identity_reported);
        handler.reset_state();

        let bytes: &[u8] = &[0x1b, b'[', b'c'];

        parser.advance(&mut handler, bytes);

        assert!(handler.identity_reported);
        handler.reset_state();

        let bytes: &[u8] = &[0x1b, b'[', b'0', b'c'];

        parser.advance(&mut handler, bytes);

        assert!(handler.identity_reported);
    }

    #[test]
    fn parse_terminal_identity_esc() {
        let bytes: &[u8] = &[0x1b, b'Z'];

        let mut parser = Processor::new();
        let mut handler = MockHandler::default();

        parser.advance(&mut handler, bytes);

        assert!(handler.identity_reported);
        handler.reset_state();

        let bytes: &[u8] = &[0x1b, b'#', b'Z'];

        let mut parser = Processor::new();
        let mut handler = MockHandler::default();

        parser.advance(&mut handler, bytes);

        assert!(!handler.identity_reported);
        handler.reset_state();
    }

    #[test]
    fn parse_truecolor_attr() {
        static BYTES: &[u8] = &[
            0x1b, b'[', b'3', b'8', b';', b'2', b';', b'1', b'2', b'8', b';', b'6', b'6', b';',
            b'2', b'5', b'5', b'm',
        ];

        let mut parser = Processor::new();
        let mut handler = MockHandler::default();

        parser.advance(&mut handler, BYTES);

        let spec = Rgb {
            r: 128,
            g: 66,
            b: 255,
        };

        assert_eq!(handler.attr, Some(Attribute::Foreground(Color::Spec(spec))));
    }

    /// No exactly a test; useful for debugging.
    #[test]
    fn parse_zsh_startup() {
        static BYTES: &[u8] = &[
            0x1b, b'[', b'1', b'm', 0x1b, b'[', b'7', b'm', b'%', 0x1b, b'[', b'2', b'7', b'm',
            0x1b, b'[', b'1', b'm', 0x1b, b'[', b'0', b'm', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', b' ', b' ', b'\r', b' ', b'\r', b'\r', 0x1b, b'[', b'0', b'm', 0x1b, b'[', b'2',
            b'7', b'm', 0x1b, b'[', b'2', b'4', b'm', 0x1b, b'[', b'J', b'j', b'w', b'i', b'l',
            b'm', b'@', b'j', b'w', b'i', b'l', b'm', b'-', b'd', b'e', b's', b'k', b' ', 0x1b,
            b'[', b'0', b'1', b';', b'3', b'2', b'm', 0xe2, 0x9e, 0x9c, b' ', 0x1b, b'[', b'0',
            b'1', b';', b'3', b'2', b'm', b' ', 0x1b, b'[', b'3', b'6', b'm', b'~', b'/', b'c',
            b'o', b'd', b'e',
        ];

        let mut handler = MockHandler::default();
        let mut parser = Processor::new();

        parser.advance(&mut handler, BYTES);
    }

    #[test]
    fn parse_designate_g0_as_line_drawing() {
        static BYTES: &[u8] = &[0x1b, b'(', b'0'];
        let mut parser = Processor::new();
        let mut handler = MockHandler::default();

        parser.advance(&mut handler, BYTES);

        assert_eq!(handler.index, CharsetIndex::G0);
        assert_eq!(handler.charset, Charset::SpecialCharacterAndLineDrawing);
    }

    #[test]
    fn parse_designate_g1_as_line_drawing_and_invoke() {
        static BYTES: &[u8] = &[0x1b, b')', b'0', 0x0e];
        let mut parser = Processor::new();
        let mut handler = MockHandler::default();

        parser.advance(&mut handler, &BYTES[..3]);

        assert_eq!(handler.index, CharsetIndex::G1);
        assert_eq!(handler.charset, Charset::SpecialCharacterAndLineDrawing);

        let mut handler = MockHandler::default();
        parser.advance(&mut handler, &[BYTES[3]]);

        assert_eq!(handler.index, CharsetIndex::G1);
    }

    #[test]
    fn parse_valid_rgb_colors() {
        assert_eq!(
            xparse_color(b"rgb:f/e/d"),
            Some(Rgb {
                r: 0xff,
                g: 0xee,
                b: 0xdd
            })
        );
        assert_eq!(
            xparse_color(b"rgb:11/aa/ff"),
            Some(Rgb {
                r: 0x11,
                g: 0xaa,
                b: 0xff
            })
        );
        assert_eq!(
            xparse_color(b"rgb:f/ed1/cb23"),
            Some(Rgb {
                r: 0xff,
                g: 0xec,
                b: 0xca
            })
        );
        assert_eq!(
            xparse_color(b"rgb:ffff/0/0"),
            Some(Rgb {
                r: 0xff,
                g: 0x0,
                b: 0x0
            })
        );
    }

    #[test]
    fn parse_valid_legacy_rgb_colors() {
        assert_eq!(
            xparse_color(b"#1af"),
            Some(Rgb {
                r: 0x10,
                g: 0xa0,
                b: 0xf0
            })
        );
        assert_eq!(
            xparse_color(b"#11aaff"),
            Some(Rgb {
                r: 0x11,
                g: 0xaa,
                b: 0xff
            })
        );
        assert_eq!(
            xparse_color(b"#110aa0ff0"),
            Some(Rgb {
                r: 0x11,
                g: 0xaa,
                b: 0xff
            })
        );
        assert_eq!(
            xparse_color(b"#1100aa00ff00"),
            Some(Rgb {
                r: 0x11,
                g: 0xaa,
                b: 0xff
            })
        );
    }

    #[test]
    fn parse_invalid_rgb_colors() {
        assert_eq!(xparse_color(b"rgb:0//"), None);
        assert_eq!(xparse_color(b"rgb://///"), None);
    }

    #[test]
    fn parse_invalid_legacy_rgb_colors() {
        assert_eq!(xparse_color(b"#"), None);
        assert_eq!(xparse_color(b"#f"), None);
    }

    #[test]
    fn parse_invalid_number() {
        assert_eq!(parse_number(b"1abc"), None);
    }

    #[test]
    fn parse_valid_number() {
        assert_eq!(parse_number(b"123"), Some(123));
    }

    #[test]
    fn parse_number_too_large() {
        assert_eq!(parse_number(b"321"), None);
    }

    #[test]
    fn set_cursor_style() {
        let bytes: &[u8] = b"\x1b[5 q";

        let mut parser = Processor::new();
        let mut handler = MockHandler::default();

        parser.advance(&mut handler, bytes);

        assert_eq!(
            handler.cursor_style,
            Some(CursorStyle {
                shape: CursorShape::Beam,
                blinking: true
            })
        )
    }

    #[test]
    fn parse_osc4_set_color() {
        let bytes: &[u8] = b"\x1b]4;0;#fff\x1b\\";

        let mut parser = Processor::new();
        let mut handler = MockHandler::default();

        parser.advance(&mut handler, bytes);

        assert_eq!(
            handler.color,
            Some(Rgb {
                r: 0xf0,
                g: 0xf0,
                b: 0xf0
            })
        );
    }

    #[test]
    fn parse_osc104_reset_color() {
        let bytes: &[u8] = b"\x1b]104;1;\x1b\\";

        let mut parser = Processor::new();
        let mut handler = MockHandler::default();

        parser.advance(&mut handler, bytes);

        assert_eq!(handler.reset_colors, vec![1]);
    }

    #[test]
    fn parse_osc104_reset_all_colors() {
        let bytes: &[u8] = b"\x1b]104;\x1b\\";

        let mut parser = Processor::new();
        let mut handler = MockHandler::default();

        parser.advance(&mut handler, bytes);

        let expected: Vec<usize> = (0..256).collect();
        assert_eq!(handler.reset_colors, expected);
    }

    #[test]
    fn parse_osc104_reset_all_colors_no_semicolon() {
        let bytes: &[u8] = b"\x1b]104\x1b\\";

        let mut parser = Processor::new();
        let mut handler = MockHandler::default();

        parser.advance(&mut handler, bytes);

        let expected: Vec<usize> = (0..256).collect();
        assert_eq!(handler.reset_colors, expected);
    }
}
