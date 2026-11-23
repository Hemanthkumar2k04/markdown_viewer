use gpui_kit::base::StyledExt;
use gpui_kit::base::input::{EditorState, InputEvent};
use gpui_kit::component::input::Editor;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use gpui_kit::component::{
    ActiveTheme, IconName, Sizable, Theme, ThemeMode, ThemeRegistry, TitleBar,
    button::{Button, ButtonVariants},
    label::Label,
    sidebar::{Sidebar, SidebarCollapsible, SidebarMenu, SidebarMenuItem},
    text::TextView,
};

const TITLEBAR_EXPANDED: Pixels = px(36.);
const TITLEBAR_PEEK: Pixels = px(8.);

struct MarkdownViewer {
    titlebar_hovered: bool,
    sidebar_open: bool,
    content: Option<SharedString>,
    editor: Entity<EditorState>,
    _editor_subscription: Subscription,
    edit_generation: u64,
}

impl MarkdownViewer {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
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
                            cx.notify();
                        }
                    });
                })
                .detach();
            }
        });
        Self {
            titlebar_hovered: false,
            sidebar_open: false,
            content: None,
            editor,
            _editor_subscription,
            edit_generation: 0,
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
        cx.spawn_in(window, async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { std::fs::read_to_string(path) })
                .await;
            _ = this.update_in(cx, |this, window, cx| {
                let text: SharedString = match result {
                    Ok(text) => text.into(),
                    Err(err) => format!("Failed to read file: {err}").into(),
                };
                this.content = Some(text.clone());
                editor.update(cx, |state, cx| state.set_value(text, window, cx));
                cx.notify();
            });
        })
        .detach();
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
        div().h_flex().w_full().px_3().py_2().child(
            Button::new("burger")
                .ghost()
                .small()
                .icon(IconName::Menu)
                .tooltip("Toggle sidebar")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.sidebar_open = !this.sidebar_open;
                    cx.notify();
                })),
        )
    }

    fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let open = self.sidebar_open;

        div()
            .h_full()
            .pl_2()
            .py_2()
            .when(open, |el| el.pr_2())
            .child(
                Sidebar::new("main-sidebar")
                    .w(px(220.))
                    .rounded_lg()
                    .border_1()
                    .border_color(cx.theme().border)
                    .collapsible(SidebarCollapsible::Offcanvas)
                    .collapsed(!open)
                    .child(Self::render_sidebar_menu(cx)),
            )
    }

    fn render_sidebar_menu(cx: &mut Context<Self>) -> SidebarMenu {
        SidebarMenu::new()
            .child(
                SidebarMenuItem::new("File")
                    .icon(IconName::File)
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.pick_file(window, cx);
                    })),
            )
            .child(
                SidebarMenuItem::new("View")
                    .icon(IconName::Eye)
                    .on_click(|_, _, _| {}),
            )
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
                        .gap_2()
                        .child(Label::new("Open a file to view"))
                        .child(Button::new("open-btn").label("Choose a file").on_click(
                            cx.listener(|this, _, window, cx| {
                                this.pick_file(window, cx);
                            }),
                        )),
                )
                .into_any_element(),
        };

        div()
            .flex()
            .flex_row()
            .flex_1()
            .w_full()
            .min_h_0()
            .child(self.render_sidebar(cx))
            .child(content)
    }
}

impl Render for MarkdownViewer {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(cx.theme().background)
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
    application().with_assets(assets::Assets).run(|cx| {
        init(cx);

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
        Theme::update(cx, |theme| theme.dark_theme = config);
        Theme::change(ThemeMode::Dark, None, cx);

        open_window(
            WindowOptions {
                ..TitleBar::window_options()
            },
            cx,
            |window, cx| cx.new(|cx| MarkdownViewer::new(window, cx)),
        )
        .expect("Failed to open window");
    });
}
