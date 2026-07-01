//! Rustploy Remote (glacier-ui) — desktop client whose UI is described in KDL
//! templates and rendered by the published `glacier-ui` engine. The network
//! layer runs through glacier-ui's async bridge (effects + subscriptions).

mod app;

use app::App;
use iced::Font;

fn main() -> iced::Result {
    iced::application(App::boot, App::update, App::view)
        .title("Rustploy Remote")
        .subscription(App::subscription)
        .theme(App::theme)
        .font(include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf").as_slice())
        .font(include_bytes!("../assets/fonts/JetBrainsMono-Bold.ttf").as_slice())
        .default_font(Font::with_name("JetBrains Mono"))
        .exit_on_close_request(false)
        .window(app::window_settings())
        .run()
}
