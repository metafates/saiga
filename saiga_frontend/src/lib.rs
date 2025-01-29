pub mod backend;
pub mod font;
pub mod settings;
pub mod terminal;
pub mod theme;

use iced::widget::{button, text};
use iced::Element;

pub fn run() -> iced::Result {
    iced::run("Saiga", update, view)
}

#[derive(Debug, Clone)]
enum Message {
    Increment,
}

fn update(counter: &mut u64, message: Message) {
    match message {
        Message::Increment => *counter += 1,
    }
}

fn view(counter: &u64) -> Element<Message> {
    button(text(counter)).on_press(Message::Increment).into()
}
