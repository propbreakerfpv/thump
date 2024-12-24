use std::{sync::mpsc::Sender, time::Duration};

use iced::{
    advanced::{layout::Node, renderer, Widget},
    border,
    event::Status,
    Color, Element, Point, Rectangle, Size,
};

use crate::{Message, PlayerMessage};

#[derive(Debug, Clone, Copy)]
pub struct SeekPos {
    /// the position in the current track as map to 0 to 1
    seek_pos: f64,
}

pub struct Seeker {
    seeker_pos: SeekPos,
    curson_pos: Point,
    player_tx: Sender<PlayerMessage>,
}

#[derive(Default)]
pub struct SeekerState {
    seeker_pos: f32,
    mouse_held_down: bool,
}


impl From<SeekPos> for f64 {
    fn from(val: SeekPos) -> Self {
        val.seek_pos
    }
}
// impl Into<f32> for SeekPos {
//     fn into(self) -> f32 {
//         self.seek_pos as f32
//     }
// }

impl SeekPos {
    /// create a SeekPos from a f64 representing the number of seconts that have past
    pub fn from_secs_percent(percent: f64, duration: Duration) -> SeekPos {
        SeekPos {
            seek_pos: percent / duration.as_secs_f64(),
        }
    }
    pub fn from_range(pos: f64, range_max: f64) -> SeekPos {
        SeekPos {
            seek_pos: pos / range_max,
        }
    }
    pub fn get(&self) -> f64 {
        self.seek_pos
    }
}

impl Seeker {
    fn new(seeker_pos: SeekPos, player_tx: Sender<PlayerMessage>) -> Seeker 
    {
        Seeker {
            seeker_pos,
            curson_pos: Point::new(0.0, 0.0),
            player_tx,
        }
    }
}

pub fn seeker(seeker_pos: SeekPos, player_tx: Sender<PlayerMessage>) -> Seeker {
    Seeker::new(seeker_pos, player_tx)
}

impl<T, R> Widget<Message, T, R> for Seeker
where
    R: renderer::Renderer,
{
    fn size(&self) -> iced::Size<iced::Length> {
        iced::Size {
            width: iced::Length::Shrink,
            height: iced::Length::Shrink,
        }
    }

    fn layout(
        &self,
        _tree: &mut iced::advanced::widget::Tree,
        _renderer: &R,
        _limits: &iced::advanced::layout::Limits,
    ) -> iced::advanced::layout::Node {
        Node::new(Size::new(200.0, 12.0))
    }

    fn draw(
        &self,
        _tree: &iced::advanced::widget::Tree,
        renderer: &mut R,
        _theme: &T,
        _style: &renderer::Style,
        layout: iced::advanced::Layout<'_>,
        _cursor: iced::advanced::mouse::Cursor,
        _viewport: &iced::Rectangle,
    ) {
        let bounds = layout.bounds();

        let thickniss = 5.0;
        let thumb_radios = 7.5;
        let seeker_width = 200.0;

        let progress = Rectangle::new(
            Point::new(bounds.x, bounds.y + (thumb_radios - (thickniss / 2.0))),
            Size::new(seeker_width, thickniss),
        );
        renderer.fill_quad(
            renderer::Quad {
                bounds: progress,
                border: border::rounded(thickniss),
                ..renderer::Quad::default()
            },
            Color::from_rgba(120.0, 120.0, 120.0, 0.2),
        );

        let thumb = Rectangle::new(
            Point::new(self.seeker_pos.seek_pos as f32 * seeker_width - thumb_radios, bounds.y),
            Size::new(thumb_radios * 2.0, thumb_radios * 2.0),
        );
        renderer.fill_quad(
            renderer::Quad {
                bounds: thumb,
                border: border::rounded(thumb_radios),
                ..renderer::Quad::default()
            },
            Color::from_rgba(0.2, 0.0, 1.0, 1.0),
        );
    }

    fn tag(&self) -> iced::advanced::widget::tree::Tag {
        iced::advanced::widget::tree::Tag::stateless()
    }

    fn state(&self) -> iced::advanced::widget::tree::State {
        iced::advanced::widget::tree::State::new(SeekerState::default())
    }

    fn children(&self) -> Vec<iced::advanced::widget::Tree> {
        Vec::new()
    }

    fn diff(&self, _tree: &mut iced::advanced::widget::Tree) {}

    fn operate(
        &self,
        _state: &mut iced::advanced::widget::Tree,
        _layout: iced::advanced::Layout<'_>,
        _renderer: &R,
        _operation: &mut dyn iced::advanced::widget::Operation,
    ) {
    }

    fn on_event(
        &mut self,
        state: &mut iced::advanced::widget::Tree,
        event: iced::Event,
        layout: iced::advanced::Layout<'_>,
        _cursor: iced::advanced::mouse::Cursor,
        _renderer: &R,
        _clipboard: &mut dyn iced::advanced::Clipboard,
        shell: &mut iced::advanced::Shell<'_, Message>,
        _viewport: &iced::Rectangle,
    ) -> iced::advanced::graphics::core::event::Status {
        let state: &mut SeekerState = state.state.downcast_mut();
        match event {
            iced::Event::Mouse(iced::mouse::Event::CursorMoved { position }) => {
                let bounds = layout.bounds();
                self.curson_pos = position;
                if bounds.contains(position) && state.mouse_held_down {
                    self.seeker_pos = SeekPos::from_range((self.curson_pos.x - bounds.x) as f64, 200.0);
                    // self.player_tx.send(PlayerMessage::Seek(self.seeker_pos)).expect("failed to send Seek message");
                    Status::Captured
                } else {
                    Status::Ignored
                }
            },
            iced::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)) => {
                let bounds = layout.bounds();
                state.mouse_held_down = true;
                if bounds.contains(self.curson_pos) {
                    shell.publish(Message::Seeking);
                    self.seeker_pos = SeekPos::from_range((self.curson_pos.x - bounds.x) as f64, 200.0);
                    // self.player_tx.send(PlayerMessage::Seek(self.seeker_pos)).expect("failed to send Seek message");
                    Status::Captured
                } else {
                    Status::Ignored
                }
            }
            iced::Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left)) => {
                let bounds = layout.bounds();
                shell.publish(Message::DoneSeeking);
                if bounds.contains(self.curson_pos) {
                    self.player_tx.send(PlayerMessage::Seek(self.seeker_pos)).expect("failed to send Seek message");
                }
                state.mouse_held_down = false;
                Status::Ignored
            }
            _ => Status::Ignored
        }
    }

    fn mouse_interaction(
        &self,
        _state: &iced::advanced::widget::Tree,
        _layout: iced::advanced::Layout<'_>,
        _cursor: iced::advanced::mouse::Cursor,
        _viewport: &iced::Rectangle,
        _renderer: &R,
    ) -> iced::advanced::mouse::Interaction {
        iced::advanced::mouse::Interaction::None
    }

    fn overlay<'a>(
        &'a mut self,
        _state: &'a mut iced::advanced::widget::Tree,
        _layout: iced::advanced::Layout<'_>,
        _renderer: &R,
        _translation: iced::Vector,
    ) -> Option<iced::advanced::overlay::Element<'a, Message, T, R>> {
        None
    }
}

impl<T, R> From<Seeker> for Element<'_, Message, T, R>
where
    R: renderer::Renderer,
{
    fn from(seeker: Seeker) -> Self {
        Self::new(seeker)
    }
}
