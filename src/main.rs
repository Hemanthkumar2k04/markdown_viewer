mod recent_files;
mod viewer;

use gpui_kit::component::{Theme, ThemeMode, ThemeRegistry, TitleBar};
use gpui_kit::*;

use viewer::{MarkdownViewer, Save};

fn main() {
    unsafe {
        std::env::set_var("ZED_DEVICE_ID", "25ac");
    }
    let config_dir = dirs::config_dir().expect("No Config directory found");
    let app_dir = config_dir.join("mdpane");
    std::fs::create_dir_all(&app_dir).expect("Failed to create config directory");
    let recent_files_path = app_dir.join("recent_files.json");

    application().with_assets(assets::Assets).run(|cx| {
        init(cx);

        cx.bind_keys([KeyBinding::new("ctrl-s", Save, None)]);
        {
            let registry = ThemeRegistry::global_mut(cx);
            registry
                .load_themes_from_str(include_str!("./themes/charcoal.json"))
                .expect("file not found");
        }
        let config = ThemeRegistry::global(cx)
            .themes()
            .values()
            .find(|t| t.name == "Charcoal Dark")
            .cloned()
            .expect("Theme not registered");
        let initial = size(px(1440.), px(900.));
        Theme::update(cx, |theme| theme.dark_theme = config);
        Theme::change(ThemeMode::Dark, None, cx);

        open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                    point(px(0.), px(0.)),
                    initial,
                ))),
                window_background: WindowBackgroundAppearance::Transparent,
                ..TitleBar::window_options()
            },
            cx,
            |window, cx| cx.new(|cx| MarkdownViewer::new(window, cx, recent_files_path)),
        )
        .expect("Failed to open window");
    });
}
