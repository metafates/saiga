#[derive(Clone)]
pub enum Event {
    SetTitle(String),
    PtyWrite(Vec<u8>),
}

pub trait EventListener {
    fn on_event(&self, event: Event);
}
