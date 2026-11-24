mod app;
mod data;
mod net;
mod ui;

use gpui_kit::*;
use gpui_kit::component::{Theme, ThemeMode};

fn main() {
    gpui_kit::application()
        .with_assets(gpui_kit::assets::AllAssets)
        .run(move |cx| {
            gpui_kit::init(cx);
            Theme::change(ThemeMode::Dark, None, cx);

            let options = WindowOptions {
                window_bounds: Some(WindowBounds::centered(
                    size(px(1440.), px(900.)),
                    cx,
                )),
                titlebar: Some(TitlebarOptions {
                    title: Some("Discord UI · GPUI Kit".into()),
                    ..Default::default()
                }),
                ..Default::default()
            };

            gpui_kit::open_window(options, cx, |window, cx| {
                cx.new(|cx| app::DiscordApp::new(window, cx))
            })
            .expect("failed to open window");
        });
}
