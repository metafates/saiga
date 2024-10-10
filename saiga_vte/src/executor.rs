use crate::Params;

pub trait Executor {
    /// Draw a character to the screen.
    fn print(&mut self, c: char);

    /// Execute C0 or C1 control function
    fn execute(&mut self, byte: u8);

    /// Pass bytes as part of a device control string to the handle chosen in `hook`. C0 controls
    /// will also be passed to the handler.
    fn put(&mut self, byte: u8);

    /// Invoked when a final character arrives in first part of device control string.
    ///
    /// The control function should be determined from the private marker, final character, and
    /// execute with a parameter list. A handler should be selected for remaining characters in the
    /// string; the handler function should subsequently be called by `put` for every character in
    /// the control string.
    ///
    /// The `ignore` flag indicates that more than two intermediates arrived and
    /// subsequent characters were ignored.
    fn hook(&mut self, params: &Params, intermediates: &[u8], ignore: bool, action: char);

    /// Called when a device control string is terminated.
    ///
    /// The previously selected handler should be notified that the DCS has
    /// terminated.
    fn unhook(&mut self);

    /// Dispatch an operating system command.
    fn osc_dispatch(&mut self, params: &[&[u8]], bell_terminated: bool);

    /// The final character of an escape sequence has arrived.
    ///
    /// The `ignore` flag indicates that more than two intermediates arrived and
    /// subsequent characters were ignored.
    fn esc_dispatch(&mut self, intermediates: &[u8], ignore: bool, byte: u8);

    /// A final character has arrived for a CSI sequence
    ///
    /// The `ignore` flag indicates that either more than two intermediates arrived
    /// or the number of parameters exceeded the maximum supported length,
    /// and subsequent characters were ignored.
    fn csi_dispatch(&mut self, params: &Params, intermediates: &[u8], ignore: bool, action: char);
}
