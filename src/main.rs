use std::collections::VecDeque;
use std::path::PathBuf;

use gpui_kit::base::StyledExt;
use gpui_kit::base::input::{EditorState, InputEvent};
use gpui_kit::component::ThemeStyled;
use gpui_kit::component::input::Editor;
use gpui_kit::component::menu::{DropdownMenu, PopupMenuItem};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use gpui_kit::component::{
    ActiveTheme, IconName, Sizable, Theme, ThemeMode, ThemeRegistry, TitleBar, WindowExt as _,
    button::{Button, ButtonVariants},
    label::Label,
    text::TextView,
};

const TITLEBAR_EXPANDED: Pixels = px(36.);
const TITLEBAR_PEEK: Pixels = px(8.);
const MAX_RECENT: usize = 10;
actions!(markdown_viewer, [Save]);

#[derive(Default, Clone)]
struct RecentFiles {
    entries: VecDeque<PathBuf>,
}

struct MarkdownViewer {
    titlebar_hovered: bool,
    file_path: Option<PathBuf>,
    content: Option<SharedString>,
    editor: Entity<EditorState>,
    _editor_subscription: Subscription,
    edit_generation: u64,
    dirty: bool,
    history: RecentFiles,
    config_path: PathBuf,
}
#[derive(serde::Serialize, serde::Deserialize)]
struct RecentFilesFile {
    #[serde(default)]
    version: u32,
    #[serde(default)]
    files: Vec<PathBuf>,
}

impl RecentFiles {
    fn touch(&mut self, path: PathBuf) {
        let path = std::fs::canonicalize(&path).unwrap_or(path);
        if let Some(pos) = self.entries.iter().position(|p| p == &path) {
            self.entries.remove(pos);
        }
        self.entries.push_front(path);
        self.entries.truncate(MAX_RECENT);
    }

    fn remove_missing(&mut self) {
        self.entries.retain(|p| p.exists());
    }
    fn load(path: &std::path::Path) -> Self {
        let Ok(text) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        let Ok(file) = serde_json::from_str::<RecentFilesFile>(&text) else {
            return Self::default();
        };
        if file.version > 1 {
            return Self::default();
        };
        let mut recent = Self {
            entries: file.files.into(),
        };
        recent.remove_missing();
        recent
    }
    fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        let file = RecentFilesFile {
            version: 1,
            files: self.entries.iter().cloned().collect(),
        };
        let json = serde_json::to_string_pretty(&file).map_err(std::io::Error::other)?;
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, json.as_bytes())?;
        std::fs::rename(tmp, path)?;
        Ok(())
    }
}

impl MarkdownViewer {
    fn new(window: &mut Window, cx: &mut Context<Self>, config_path: PathBuf) -> Self {
        let editor = cx.new(|cx| EditorState::new(window, cx));
        let _editor_subscription = cx.subscribe(&editor, |this, editor, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                this.edit_generation = this.edit_generation.wrapping_add(1);
                let generation = this.edit_generation;
                let editor = editor.clone();
                cx.spawn(async move |this, cx| {
                    cx.background_executor()
                        .timer(std::time::Duration::from_millis(150))
                        .await;
                    _ = this.update(cx, |this, cx| {
                        // Only the latest keystroke wins.
                        if this.edit_generation == generation {
                            this.content = Some(editor.read(cx).value());
                            this.dirty = true;
                            cx.notify();
                        }
                    });
                })
                .detach();
            }
        });
        let history = RecentFiles::load(&config_path);
        Self {
            titlebar_hovered: false,
            content: None,
            editor,
            _editor_subscription,
            edit_generation: 0,
            dirty: false,
            file_path: None,
            history,
            config_path,
        }
    }

    fn pick_file(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Open a Markdown file".into()),
        });

        cx.spawn_in(window, async move |this, cx| match receiver.await {
            Ok(Ok(Some(paths))) => {
                if let Some(path) = paths.into_iter().next() {
                    _ = this.update_in(cx, |this, window, cx| this.load_file(path, window, cx));
                }
            }
            Ok(Ok(None)) => {}
            Ok(Err(err)) => {
                _ = this.update_in(cx, |this, _window, cx| {
                    this.content = Some(format!("Failed to open file dialog: {err}").into());
                    cx.notify();
                });
            }
            Err(_) => {}
        })
        .detach();
    }

    fn load_file(&mut self, path: std::path::PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        let editor = self.editor.clone();
        let read_path = path.clone();

        cx.spawn_in(window, async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { std::fs::read_to_string(read_path) })
                .await;
            _ = this.update_in(cx, |this, window, cx| {
                match result {
                    Ok(text) => {
                        let text: SharedString = text.into();
                        this.file_path = Some(path.clone());
                        this.dirty = false;
                        this.history.touch(path);
                        this.persist_history(window, cx);
                        this.content = Some(text.clone());
                        editor.update(cx, |state, cx| state.set_value(text, window, cx));
                    }
                    Err(err) => {
                        this.show_error(format!("Failed to read file: {err}"), window, cx);
                    }
                }

                cx.notify();
            });
        })
        .detach();
    }
    fn save_file(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(path) = self.file_path.clone() else {
            return;
        };
        let Some(text) = self.content.clone() else {
            return;
        };
        cx.spawn_in(window, async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { std::fs::write(&path, text.as_bytes()) })
                .await;

            _ = this.update_in(cx, |this, window, cx| {
                match result {
                    Ok(()) => this.dirty = false,
                    Err(err) => {
                        this.show_error(format!("Failed to save: {err}"), window, cx);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }
    fn show_error(
        &self,
        message: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let message: SharedString = message.into();
        window.open_alert_dialog(cx, move |alert, _, _| {
            alert
                .title("Error")
                .description(message.clone())
                .ok_text("OK")
                .show_cancel(false)
        });
    }
    fn render_titlebar(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let target = if self.titlebar_hovered {
            TITLEBAR_EXPANDED
        } else {
            TITLEBAR_PEEK
        };

        div()
            .id("titlebar")
            .w_full()
            .overflow_hidden()
            .on_hover(cx.listener(|this, hovered: &bool, _, cx| {
                if this.titlebar_hovered != *hovered {
                    this.titlebar_hovered = *hovered;
                    cx.notify();
                }
            }))
            .when(self.titlebar_hovered, |el| {
                el.child(TitleBar::new().child(Label::new("Markdown Viewer").text_sm()))
            })
            .with_spring(
                "titlebar-height",
                SpringAnimation::new(SpringConfig::new(320.0, 34.0, 1.0)).to(target),
                |el, height| el.h(height),
            )
    }

    fn render_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let weak_this = cx.weak_entity();

        div()
            .h_flex()
            .w_full()
            .px_3()
            .py_2()
            .gap_4()
            .child(
                Button::new("burger")
                    .ghost()
                    .small()
                    .icon(IconName::Menu)
                    .tooltip("Menu")
                    .dropdown_menu(move |menu, _, _| {
                        let weak_this = weak_this.clone();
                        menu.item(PopupMenuItem::new("File").icon(IconName::File).on_click(
                            move |_, window, cx| {
                                _ = weak_this.update(cx, |this, cx| {
                                    this.pick_file(window, cx);
                                });
                            },
                        ))
                        .item(
                            PopupMenuItem::new("View")
                                .icon(IconName::Eye)
                                .on_click(|_, _, _| {}),
                        )
                    }),
            )
            .when(self.content.is_some(), |el| {
                el.child(
                    div()
                        .px_2()
                        .py_1()
                        .rounded_full_style(cx)
                        .border_1()
                        .when_else(
                            self.dirty,
                            |el| el.border_color(cx.theme().warning.opacity(0.35)),
                            |el| el.border_color(cx.theme().success.opacity(0.35)),
                        )
                        .child(if !self.dirty {
                            Label::new("Saved").text_color(cx.theme().success.opacity(0.7))
                        } else {
                            Label::new("Not Saved").text_color(cx.theme().warning.opacity(0.7))
                        }),
                )
            })
    }

    fn render_body(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let content = match &self.content {
            Some(content) => div()
                .flex()
                .flex_row()
                .flex_1()
                .h_full()
                .min_w_0()
                .child(
                    div()
                        .flex_1()
                        .h_full()
                        .min_w_0()
                        .child(Editor::new(&self.editor).h(relative(1.)).bordered(false)),
                )
                .child(div().w(px(1.)).h_full().bg(cx.theme().border))
                .child(div().flex_1().h_full().min_w_0().p_4().child(
                    TextView::markdown("markdown-content", content.clone()).scrollable(true),
                ))
                .into_any_element(),
            None => div()
                .flex_1()
                .h_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(cx.theme().muted_foreground)
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap_4()
                        .when(!self.history.entries.is_empty(), |el| {
                            el.child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .w(px(420.))
                                    .child(Label::new("Recent files").text_sm())
                                    .children(self.history.entries.iter().cloned().map(|path| {
                                        let name = path
                                            .file_name()
                                            .map(|name| name.to_string_lossy().into_owned())
                                            .unwrap_or_else(|| path.display().to_string());

                                        Button::new(format!("recent-{}", path.display()))
                                            .ghost()
                                            .w_full()
                                            .label(name)
                                            .on_click(cx.listener(move |this, _, window, cx| {
                                                this.load_file(path.clone(), window, cx);
                                            }))
                                    })),
                            )
                        })
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .items_center()
                                .gap_2()
                                .child(Label::new("Open a file to view"))
                                .child(Button::new("open-btn").label("Choose a file").on_click(
                                    cx.listener(|this, _, window, cx| {
                                        this.pick_file(window, cx);
                                    }),
                                )),
                        ),
                )
                .into_any_element(),
        };

        div().flex().flex_1().w_full().min_h_0().child(content)
    }
    fn persist_history(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let config_path = self.config_path.clone();
        let snapshot = self.history.clone();

        cx.spawn_in(window, async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { snapshot.save(&config_path) })
                .await;
            if let Err(err) = result {
                _ = this.update_in(cx, |this, window, cx| {
                    this.show_error(format!("Failed to save history: {err}"), window, cx);
                });
            }
        })
        .detach();
    }
}

impl Render for MarkdownViewer {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("root")
            .on_action(cx.listener(|this, _: &Save, window, cx| {
                this.save_file(window, cx);
            }))
            .flex()
            .flex_col()
            .size_full()
            .text_color(cx.theme().foreground)
            .child(self.render_titlebar(cx))
            .child(self.render_toolbar(cx))
            .child(self.render_body(cx))
    }
}

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
