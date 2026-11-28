#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod recent_files;
mod viewer;

use gpui_kit::component::{Theme, ThemeMode, ThemeRegistry, TitleBar};
use gpui_kit::*;

use viewer::{MarkdownViewer, Save};

fn resolve_app_paths() -> (Option<std::path::PathBuf>, Option<std::path::PathBuf>) {
    let Some(config_dir) = dirs::config_dir() else {
        eprintln!("Warning: No config directory found; running without persistence.");
        return (None, None);
    };
    let app_dir = config_dir.join("mdpane");
    if let Err(err) = std::fs::create_dir_all(&app_dir) {
        eprintln!(
            "Warning: Failed to create config directory {}: {err}; running without persistence.",
            app_dir.display()
        );
        return (None, None);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&app_dir, std::fs::Permissions::from_mode(0o700));
    }

    (
        Some(app_dir.join("recent_files.json")),
        Some(app_dir.join("config.json")),
    )
}

fn main() {
    let (recent_files_path, config_path) = resolve_app_paths();

    application().with_assets(assets::Assets).run(|cx| {
        init(cx);

        #[cfg(target_os = "macos")]
        cx.bind_keys([KeyBinding::new("cmd-s", Save, None)]);
        #[cfg(not(target_os = "macos"))]
        cx.bind_keys([KeyBinding::new("ctrl-s", Save, None)]);

        {
            let registry = ThemeRegistry::global_mut(cx);
            registry
                .load_themes_from_str(include_str!("./themes/charcoal.json"))
                .expect("Bundled theme should be valid");
        }
        let theme_config = ThemeRegistry::global(cx)
            .themes()
            .values()
            .find(|t| t.name == "Charcoal Dark")
            .cloned()
            .expect("Bundled theme should be registered");
        let initial = size(px(1440.), px(900.));
        Theme::update(cx, |theme| theme.dark_theme = theme_config);
        Theme::change(ThemeMode::Dark, None, cx);

        open_window(
            WindowOptions {
                app_id: Some("mdpane".into()),
                window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                    point(px(0.), px(0.)),
                    initial,
                ))),
                window_background: WindowBackgroundAppearance::Transparent,
                ..TitleBar::window_options()
            },
            cx,
            |window, cx| {
                cx.new(|cx| MarkdownViewer::new(window, cx, recent_files_path, config_path))
            },
        )
        .expect("Failed to open window");
    });
}
