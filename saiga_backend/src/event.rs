#[derive(Clone, Debug)]
pub enum Event {
    SetTitle(String),

    PtyWrite(Vec<u8>),
    Bell,
}

pub trait EventListener {
    fn on_event(&self, event: Event);
}
