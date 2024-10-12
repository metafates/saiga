pub enum Event {
    SetTitle(String),
    PtyWrite(String),
}

pub trait EventListener {
    fn event(&self, event: Event);
}
