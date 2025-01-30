use std::ops::Add as _;

use iced::advanced::graphics::core::Element;
use iced::font::{Family, Stretch, Weight};
use iced::keyboard::Modifiers;
use iced::widget::container;
use iced::{Font, Length, Size, Subscription, Task, Theme, window};
use iced_saiga::{Command, TermMode, TermView};

pub fn run() -> iced::Result {
    iced::application(App::title, App::update, App::view)
        .antialiasing(false)
        .window_size(Size {
            width: 1280.0,
            height: 720.0,
        })
        .subscription(App::subscription)
        .run_with(App::new)
}

#[derive(Debug, Clone)]
pub enum Event {
    Terminal(iced_saiga::Event),
    FontSize(f32),
}

struct App {
    title: String,
    term: iced_saiga::Terminal,
    font_settings: iced_saiga::settings::FontSettings,
}

impl App {
    fn new() -> (Self, Task<Event>) {
        let system_shell = std::env::var("SHELL")
            .expect("SHELL variable is not defined")
            .to_string();
        let term_id = 0;
        let term_settings = iced_saiga::settings::Settings {
            font: iced_saiga::settings::FontSettings {
                size: 17.0,
                font_type: Font {
                    weight: Weight::Bold,
                    family: Family::Name("JetBrainsMono Nerd Font Mono"),
                    stretch: Stretch::Normal,
                    ..Default::default()
                },
                ..Default::default()
            },
            theme: iced_saiga::settings::ThemeSettings::default(),
            backend: iced_saiga::settings::BackendSettings {
                shell: system_shell.to_string(),
            },
        };

        let font_settings = term_settings.font.clone();

        (
            Self {
                title: String::from("Saiga"),
                term: iced_saiga::Terminal::new(term_id, term_settings),
                font_settings,
            },
            Task::none(),
        )
    }

    fn title(&self) -> String {
        self.title.clone()
    }

    fn subscription(&self) -> Subscription<Event> {
        // TODO: make it work somehow. Currently, terminal captures it itself, so not working
        let key_subscription = iced::keyboard::on_key_press(|key, m| match key {
            iced::keyboard::Key::Character(c) if m.contains(Modifiers::COMMAND) => match c.as_str()
            {
                "=" => Some(Event::FontSize(2.0)),
                "-" => Some(Event::FontSize(-2.0)),
                _ => None,
            },
            _ => None,
        });

        let term_subscription = iced_saiga::Subscription::new(self.term.id);
        let term_event_stream = term_subscription.event_stream();

        Subscription::batch(vec![
            key_subscription,
            Subscription::run_with_id(self.term.id, term_event_stream).map(Event::Terminal),
        ])
    }

    fn update(&mut self, event: Event) -> Task<Event> {
        match event {
            Event::Terminal(iced_saiga::Event::CommandReceived(_, cmd)) => {
                match self.term.update(cmd) {
                    iced_saiga::actions::Action::Shutdown => {
                        window::get_latest().and_then(window::close)
                    }
                    iced_saiga::actions::Action::ChangeTitle(title) => {
                        self.title = title;

                        Task::none()
                    }
                    _ => Task::none(),
                }
            }
            Event::FontSize(delta) => {
                self.font_settings.size = self.font_settings.size.add(delta).max(5.0);
                self.term
                    .update(iced_saiga::Command::ChangeFont(self.font_settings.clone()));

                Task::none()
            }
        }
    }

    fn view(&self) -> Element<Event, Theme, iced::Renderer> {
        container(TermView::show(&self.term).map(Event::Terminal))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
