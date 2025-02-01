use iced::{
    alignment::{Horizontal, Vertical},
    keyboard::{Key, Modifiers},
    widget::container,
    Element, Length, Point, Rectangle, Size, Theme,
};
use iced_core::{
    clipboard::Kind as ClipboardKind,
    text::{LineHeight, Shaping},
    widget::{operation, tree, Tree},
    Widget,
};
use iced_graphics::geometry::{Path, Text};
use saiga_backend::term::{cell, TermMode};

use crate::{
    backend::BackendCommand,
    bindings::{BindingAction, InputKind},
    terminal::{Command, Event, Terminal},
    theme::TerminalStyle as _,
};
use iced::mouse::Cursor;

pub struct TermView<'a> {
    term: &'a Terminal,
}

impl<'a> TermView<'a> {
    pub fn show(term: &'a Terminal) -> Element<'a, Event> {
        container(Self { term })
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_| term.theme.container_style())
            .into()
    }

    fn handle_keyboard_event(
        &self,
        state: &mut TermViewState,
        clipboard: &mut dyn iced_graphics::core::Clipboard,
        event: iced::keyboard::Event,
    ) -> Option<Command> {
        let Some(ref backend) = self.term.backend else {
            return None;
        };

        let mut binding_action = BindingAction::Ignore;
        let last_content = backend.renderable_content();

        if let iced::keyboard::Event::KeyPressed {
            key,
            modifiers,
            text,
            ..
        } = event
        {
            match key {
                Key::Character(_) => {
                    if let Some(c) = text {
                        binding_action = self.term.bindings.get_action(
                            InputKind::Char(c.to_ascii_lowercase()),
                            state.keyboard_modifiers,
                            last_content.term_mode,
                        );

                        if binding_action == BindingAction::Ignore {
                            return Some(Command::ProcessBackendCommand(BackendCommand::Write(
                                c.as_bytes().to_vec(),
                            )));
                        }
                    }
                }
                Key::Named(code) => {
                    binding_action = self.term.bindings.get_action(
                        InputKind::KeyCode(code),
                        modifiers,
                        last_content.term_mode,
                    );
                }
                _ => {}
            }
        }

        match binding_action {
            BindingAction::Char(c) => {
                let mut buf = [0; 4];
                let str = c.encode_utf8(&mut buf);

                Some(Command::ProcessBackendCommand(BackendCommand::Write(
                    str.as_bytes().to_vec(),
                )))
            }
            BindingAction::Esc(seq) => Some(Command::ProcessBackendCommand(BackendCommand::Write(
                seq.as_bytes().to_vec(),
            ))),
            BindingAction::Paste => {
                if let Some(data) = clipboard.read(ClipboardKind::Standard) {
                    let input: Vec<u8> = data.bytes().collect();

                    Some(Command::ProcessBackendCommand(BackendCommand::Write(input)))
                } else {
                    None
                }
            }
            BindingAction::Copy => {
                // clipboard.write(ClipboardKind::Standard, backend.selectable_content());
                None
            }
            _ => None,
        }
    }
}

pub struct TermViewState {
    is_focused: bool,
    keyboard_modifiers: Modifiers,
    size: Size<f32>,
}

impl Default for TermViewState {
    fn default() -> Self {
        Self {
            is_focused: true,
            keyboard_modifiers: Modifiers::empty(),
            size: Size::from([0.0, 0.0]),
        }
    }
}

impl operation::Focusable for TermViewState {
    fn is_focused(&self) -> bool {
        self.is_focused
    }

    fn focus(&mut self) {
        self.is_focused = true;
    }

    fn unfocus(&mut self) {
        self.is_focused = false;
    }
}

impl Widget<Event, Theme, iced::Renderer> for TermView<'_> {
    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Fill,
            height: Length::Fill,
        }
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<TermViewState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(TermViewState::default())
    }

    fn layout(
        &self,
        _tree: &mut Tree,
        _renderer: &iced::Renderer,
        limits: &iced_core::layout::Limits,
    ) -> iced_core::layout::Node {
        let size = limits.resolve(Length::Fill, Length::Fill, Size::ZERO);
        iced::advanced::layout::Node::new(size)
    }

    fn operate(
        &self,
        tree: &mut Tree,
        _layout: iced_core::Layout<'_>,
        _renderer: &iced::Renderer,
        operation: &mut dyn operation::Operation,
    ) {
        let state = tree.state.downcast_mut::<TermViewState>();
        let wid = iced_core::widget::Id::from(self.term.widget_id());
        operation.focusable(state, Some(&wid));
    }

    fn draw(
        &self,
        _tree: &Tree,
        renderer: &mut iced::Renderer,
        _theme: &Theme,
        _style: &iced_core::renderer::Style,
        layout: iced_core::Layout<'_>,
        _cursor: iced_core::mouse::Cursor,
        viewport: &iced::Rectangle,
    ) {
        let Some(ref backend) = &self.term.backend else {
            return;
        };

        // let _state = tree.state.downcast_ref::<TermViewState>();
        let content = backend.renderable_content();
        let term_size = content.term_size;
        let cell_width = term_size.cell_width as f32;
        let cell_height = term_size.cell_height as f32;
        let font_size = self.term.font.size;
        let font_scale_factor = self.term.font.scale_factor;
        let layout_offset_x = layout.position().x;
        let layout_offset_y = layout.position().y;

        let show_cursor = content.term_mode.contains(TermMode::SHOW_CURSOR);

        let geom = self.term.cache.draw(renderer, viewport.size(), |frame| {
            for indexed in content.grid.display_iter() {
                let x = layout_offset_x + (indexed.point.column.0 as f32 * cell_width);
                let y = layout_offset_y
                    + ((indexed.point.line.0 as f32 + content.grid.display_offset() as f32)
                        * cell_height);

                let mut fg = self.term.theme.get_color(indexed.fg);
                let mut bg = self.term.theme.get_color(indexed.bg);

                // Handle dim, inverse, and selected text
                if indexed
                    .cell
                    .flags
                    .intersects(cell::Flags::DIM | cell::Flags::DIM_BOLD)
                {
                    fg.a *= 0.7;
                }
                if indexed.cell.flags.contains(cell::Flags::INVERSE)
                    || content
                        .selectable_range
                        .is_some_and(|r| r.contains(indexed.point))
                {
                    std::mem::swap(&mut fg, &mut bg);
                }

                let cell_size = Size::new(cell_width, cell_height);

                // Draw cell background
                let background = Path::rectangle(Point::new(x, y), cell_size);
                frame.fill(&background, bg);

                // Handle cursor rendering
                if show_cursor && content.grid.cursor.point == indexed.point {
                    let cursor_color = self.term.theme.get_color(content.cursor.fg);
                    let cursor_rect = Path::rectangle(Point::new(x, y), cell_size);
                    frame.fill(&cursor_rect, cursor_color);
                }

                // Draw text
                if indexed.c != ' ' && indexed.c != '\t' {
                    if content.grid.cursor.point == indexed.point
                        && content.term_mode.contains(TermMode::APP_CURSOR)
                    {
                        fg = bg;
                    }
                    let text = Text {
                        content: indexed.c.to_string(),
                        position: Point::new(
                            x + (cell_size.width / 2.0),
                            y + (cell_size.height / 2.0),
                        ),
                        font: self.term.font.font_type,
                        size: iced_core::Pixels(font_size),
                        color: fg,
                        horizontal_alignment: Horizontal::Center,
                        vertical_alignment: Vertical::Center,
                        shaping: Shaping::Advanced,
                        line_height: LineHeight::Relative(font_scale_factor),
                    };
                    frame.fill_text(text);
                }
            }
        });

        use iced::advanced::graphics::geometry::Renderer as _;
        renderer.draw_geometry(geom);
    }

    fn on_event(
        &mut self,
        tree: &mut Tree,
        event: iced::Event,
        layout: iced_graphics::core::Layout<'_>,
        cursor: Cursor,
        _renderer: &iced::Renderer,
        clipboard: &mut dyn iced_graphics::core::Clipboard,
        shell: &mut iced_graphics::core::Shell<'_, Event>,
        _viewport: &Rectangle,
    ) -> iced::event::Status {
        let state = tree.state.downcast_mut::<TermViewState>();
        let layout_size = layout.bounds().size();
        if state.size != layout_size && self.term.backend.is_some() {
            state.size = layout_size;
            let cmd =
                Command::ProcessBackendCommand(BackendCommand::Resize(Some(layout_size), None));

            shell.publish(Event::CommandReceived(self.term.id, cmd));
        }

        if !state.is_focused {
            return iced::event::Status::Ignored;
        }

        let commands = match event {
            // iced::Event::Mouse(mouse_event) if self.is_cursor_in_layout(cursor, layout) => {
            //     self.handle_mouse_event(
            //         state,
            //         layout.position(),
            //         cursor.position().unwrap(), // Assuming cursor position is always available here.
            //         mouse_event,
            //     )
            // }
            iced::Event::Keyboard(keyboard_event) => {
                self.handle_keyboard_event(state, clipboard, keyboard_event)
                    .into_iter() // Convert Option to iterator (0 or 1 element)
                    .collect()
            }
            _ => Vec::new(), // No commands for other events.
        };

        if !commands.is_empty() {
            for cmd in commands {
                shell.publish(Event::CommandReceived(self.term.id, cmd));
            }
            iced::event::Status::Captured
        } else {
            iced::event::Status::Ignored
        }
    }
}

impl<'a> From<TermView<'a>> for Element<'a, Event, Theme, iced::Renderer> {
    fn from(widget: TermView<'a>) -> Self {
        Self::new(widget)
    }
}
