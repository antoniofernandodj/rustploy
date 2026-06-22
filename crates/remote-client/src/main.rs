//! Rustploy Remote — iced desktop client that mirrors the TUI, driving a
//! `rustployd` instance over the RWP (Rustploy Wire Protocol) TCP channel.

mod model;
mod rwp;
#[cfg(test)]
mod smoke;
mod store;
mod update;
mod view;
mod worker;

use iced::{Subscription, Task};
pub use model::*;

fn main() -> iced::Result {
    iced::application("Rustploy Remote", App::update, App::view)
        .subscription(App::subscription)
        .theme(|_| iced::Theme::Dark)
        .window(iced::window::Settings {
            size: iced::Size::new(1180.0, 760.0),
            // Abaixo disso o layout (titlebar, cards) começa a quebrar feio.
            min_size: Some(iced::Size::new(960.0, 600.0)),
            ..Default::default()
        })
        .run_with(App::boot)
}

impl App {
    fn boot() -> (Self, Task<Message>) {
        let default_address = shared::RustployConfig::global().rwp_address();
        (App::with_prefs(default_address, store::RemotePrefs::load()), Task::none())
    }

    fn subscription(&self) -> Subscription<Message> {
        let mut subs = vec![iced::time::every(std::time::Duration::from_secs(1))
            .map(|_| Message::Tick)];
        if let Some(s) = &self.session {
            subs.push(Subscription::run_with_id(
                self.connect_seq,
                worker::connect(s.addr.clone(), s.token.clone()),
            ));
        }
        Subscription::batch(subs)
    }

    fn view(&self) -> iced::Element<'_, Message> {
        view::view(self)
    }
}
