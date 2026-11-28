use gpui_kit::base::StyledExt;
use gpui_kit::component::input::Editor;
use gpui_kit::component::popover::Popover;
use gpui_kit::component::select::{SearchableVec, Select, SelectEvent, SelectState};
use gpui_kit::component::{
    ActiveTheme, Icon, IconName, IndexPath, Sizable, ThemeStyled, TitleBar, WindowExt as _,
    button::{Button, ButtonVariants},
    dialog::DialogFooter,
    h_resizable,
    label::Label,
    resizable_panel,
    text::TextView,
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use crate::config::{DEFAULT_EDITOR_FONT_SIZE, MAX_EDITOR_FONT_SIZE, MIN_EDITOR_FONT_SIZE};

use super::{MarkdownViewer, Save};

const GEAR_ICON: &[u8] = include_bytes!("../icons/gear.svg");
const HOME_ICON: &[u8] = include_bytes!("../icons/home.svg");

fn is_safe_external_url(url: &str) -> bool {
    let url = url.trim();
    url.get(..8)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("https://"))
        || url
            .get(..7)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("http://"))
}

impl MarkdownViewer {
    fn render_titlebar(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let weak_this = cx.weak_entity();

        div()
            .relative()
            .w_full()
            .child(
                TitleBar::new().h(px(44.)).child(
                    div()
                        .h_flex()
                        .items_center()
                        .gap_2()
                        .child(
                            Popover::new("burger-menu")
                                .anchor(Anchor::TopLeft)
                                .p_1p5()
                                .trigger(
                                    Button::new("burger")
                                        .ghost()
                                        .icon(IconName::Menu)
                                        .tooltip("Menu"),
                                )
                                .content(move |_, _, cx| {
                                    let weak_this_file = weak_this.clone();
                                    let weak_this_cfg = weak_this.clone();
                                    let popover = cx.weak_entity();
                                    let popover_cfg = popover.clone();

                                    div()
                                        .v_flex()
                                        .gap(px(5.))
                                        .min_w(px(140.))
                                        .child(
                                            Button::new("menu-file")
                                                .ghost()
                                                .w_full()
                                                .child(
                                                    div()
                                                        .h_flex()
                                                        .w_full()
                                                        .items_center()
                                                        .justify_start()
                                                        .gap_2()
                                                        .child(Icon::new(IconName::File).size_4())
                                                        .child(Label::new("File").text_base()),
                                                )
                                                .on_click(move |_, window, cx| {
                                                    _ = popover.update(cx, |state, cx| {
                                                        state.dismiss(window, cx)
                                                    });
                                                    _ = weak_this_file.update(cx, |this, cx| {
                                                        this.pick_file(window, cx);
                                                    });
                                                }),
                                        )
                                        .child(
                                            Button::new("menu-config")
                                                .ghost()
                                                .w_full()
                                                .child(
                                                    div()
                                                        .h_flex()
                                                        .w_full()
                                                        .items_center()
                                                        .justify_start()
                                                        .gap_2()
                                                        .child(
                                                            Icon::empty().data(GEAR_ICON).size_4(),
                                                        )
                                                        .child(Label::new("Config").text_base()),
                                                )
                                                .on_click(move |_, window, cx| {
                                                    _ = popover_cfg.update(cx, |state, cx| {
                                                        state.dismiss(window, cx)
                                                    });
                                                    _ = weak_this_cfg.update(cx, |this, cx| {
                                                        this.open_config_dialog(window, cx);
                                                    });
                                                }),
                                        )
                                }),
                        )
                        .child(
                            div().child(
                                Button::new("home-btn")
                                    .tooltip("Home")
                                    .icon(Icon::empty().data(HOME_ICON).large())
                                    .ghost()
                                    .on_click(cx.listener(|this, _, _window, cx| {
                                        this.file_generation = this.file_generation.wrapping_add(1);
                                        this.edit_generation = this.edit_generation.wrapping_add(1);
                                        this.content = None;
                                        this.file_path = None;
                                        this.dirty = false;
                                        cx.notify();
                                    })),
                            ),
                        ),
                ),
            )
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(Label::new("MDpane").text_sm()),
            )
    }

    fn render_body(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let content = match &self.content {
            Some(content) => h_resizable("editor-preview-split")
                .child(
                    resizable_panel().size_range(px(200.)..Pixels::MAX).child(
                        Editor::new(&self.editor)
                            .h(relative(1.))
                            .bordered(false)
                            .font_family(SharedString::from(self.config.editor.font_family.clone()))
                            .text_size(px(self.config.editor.font_size)),
                    ),
                )
                .child(
                    resizable_panel().size_range(px(200.)..Pixels::MAX).child(
                        div().size_full().p_4().child(
                            TextView::markdown("markdown-content", content.clone())
                                .scrollable(true)
                                .on_link_click(|url, _, _, cx| {
                                    if is_safe_external_url(url.as_ref()) {
                                        cx.open_url(url);
                                    }
                                }),
                        ),
                    ),
                )
                .into_any_element(),

            None => div()
                .flex_1()
                .h_full()
                .flex()
                .flex_row()
                .w_full()
                .text_color(cx.theme().muted_foreground)
                .when(!self.history.entries.is_empty(), |el| {
                    el.child(
                        div()
                            .min_w(px(300.))
                            .flex_none()
                            .h_full()
                            .flex()
                            .flex_col()
                            .justify_start()
                            .gap_8()
                            .p_10()
                            .child(Label::new("Recent files").text_2xl())
                            .child(div().flex().flex_col().gap_5().children(
                                self.history.entries.iter().map(|path| {
                                    let name = path
                                        .file_name()
                                        .map(|name| name.to_string_lossy().into_owned())
                                        .unwrap_or_else(|| path.display().to_string());
                                    let group_id: SharedString =
                                        format!("recent-row-{}", path.display()).into();
                                    let open_path = path.clone();
                                    let remove_path = path.clone();

                                    div()
                                        .relative()
                                        .w_full()
                                        .group(group_id.clone())
                                        .child(
                                            Button::new(format!("recent-{}", path.display()))
                                                .ghost()
                                                .icon(IconName::FileText)
                                                .w_full()
                                                .child(Label::new(name).text_lg())
                                                .tooltip(format!("{}", path.to_string_lossy()))
                                                .on_click(cx.listener(
                                                    move |this, _, window, cx| {
                                                        this.load_file(
                                                            open_path.clone(),
                                                            window,
                                                            cx,
                                                        );
                                                    },
                                                )),
                                        )
                                        .child(
                                            div()
                                                .absolute()
                                                .top(px(-6.))
                                                .right(px(-6.))
                                                .occlude()
                                                .invisible()
                                                .group_hover(group_id, |el| el.visible())
                                                .hover(|el| el.visible())
                                                .child(
                                                    Button::new(format!(
                                                        "remove-recent-{}",
                                                        path.display()
                                                    ))
                                                    .secondary()
                                                    .xsmall()
                                                    .icon(IconName::Close)
                                                    .tooltip("Remove from recent")
                                                    .on_click(cx.listener(
                                                        move |this, _, window, cx| {
                                                            this.remove_recent(
                                                                &remove_path,
                                                                window,
                                                                cx,
                                                            );
                                                        },
                                                    )),
                                                ),
                                        )
                                }),
                            )),
                    )
                    .child(div().w(px(1.)).h_full().bg(cx.theme().border))
                })
                .child(
                    div()
                        .flex_1()
                        .h_full()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .items_center()
                        .justify_center()
                        .gap_5()
                        .child(
                            Label::new("Open a file to view")
                                .text_3xl()
                                .mt(px(-100.))
                                .mb(px(50.)),
                        )
                        .child(
                            Button::new("open-file-btn")
                                .child(Label::new("Open a file").text_xl())
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.pick_file(window, cx);
                                }))
                                .tooltip("Select a file to open"),
                        ),
                )
                .into_any_element(),
        };

        div().flex().flex_1().w_full().min_h_0().child(content)
    }

    fn render_foot(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        div()
            .min_h(px(30.))
            .flex()
            .flex_row()
            .mx_5()
            .justify_start()
            .items_center()
            .child(div().when_some(self.file_path.as_ref(), |el, path| {
                el.child(Label::new(path.display().to_string()).text_sm())
            }))
            .child(div().when(self.content.is_some(), |el| {
                el.h_flex().w_full().px_3().py_2().gap_4().child(
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
            }))
    }

    fn open_config_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mut fonts: Vec<SharedString> = cx
            .text_system()
            .all_font_names()
            .into_iter()
            .map(SharedString::from)
            .collect();
        fonts.sort();
        fonts.dedup();

        let selected = fonts
            .iter()
            .position(|f| f.as_ref() == self.config.editor.font_family)
            .map(IndexPath::new);

        let font_select = cx.new(|cx| {
            SelectState::new(SearchableVec::new(fonts), selected, window, cx).searchable(true)
        });

        self.font_select_subscription = Some(cx.subscribe(
            &font_select,
            |this, _, event: &SelectEvent<SearchableVec<SharedString>>, cx| {
                if let SelectEvent::Confirm(Some(family)) = event {
                    this.config.editor.font_family = family.to_string();
                    this.persist_config(cx);
                    cx.notify();
                }
            },
        ));

        let weak_this = cx.weak_entity();

        window.open_dialog(cx, move |dialog, _window, cx| {
            let (current_family, current_size) = weak_this
                .upgrade()
                .map(|viewer| {
                    let editor = &viewer.read(cx).config.editor;
                    (editor.font_family.clone(), editor.font_size)
                })
                .unwrap_or_else(|| ("JetBrains Mono".to_string(), DEFAULT_EDITOR_FONT_SIZE));

            let weak_dec = weak_this.clone();
            let weak_inc = weak_this.clone();

            dialog
                .title("Config")
                .centered()
                .w(px(480.))
                .child(
                    div()
                        .v_flex()
                        .gap_4()
                        .py_2()
                        .child(
                            div()
                                .v_flex()
                                .gap_2()
                                .child(
                                    Label::new("Editor")
                                        .text_sm()
                                        .text_color(cx.theme().muted_foreground),
                                )
                                .child(div().w_full().h(px(1.)).bg(cx.theme().border)),
                        )
                        .child(
                            div()
                                .h_flex()
                                .w_full()
                                .items_center()
                                .justify_between()
                                .gap_4()
                                .child(Label::new("Font Family"))
                                .child(
                                    div().w(px(240.)).child(
                                        Select::new(&font_select)
                                            .placeholder(current_family)
                                            .menu_max_h(px(260.)),
                                    ),
                                ),
                        )
                        .child(
                            div()
                                .h_flex()
                                .w_full()
                                .items_center()
                                .justify_between()
                                .gap_4()
                                .child(Label::new("Font Size"))
                                .child(
                                    div()
                                        .h_flex()
                                        .items_center()
                                        .gap_2()
                                        .child(
                                            Button::new("editor-font-size-dec")
                                                .small()
                                                .label("-")
                                                .on_click(move |_, _, cx| {
                                                    _ = weak_dec.update(cx, |this, cx| {
                                                        this.config.editor.font_size =
                                                            (this.config.editor.font_size - 1.0)
                                                                .max(MIN_EDITOR_FONT_SIZE);
                                                        this.persist_config(cx);
                                                        cx.notify();
                                                    });
                                                }),
                                        )
                                        .child(div().min_w(px(56.)).flex().justify_center().child(
                                            Label::new(format!(
                                                "{} px",
                                                current_size.round() as u32
                                            )),
                                        ))
                                        .child(
                                            Button::new("editor-font-size-inc")
                                                .small()
                                                .label("+")
                                                .on_click(move |_, _, cx| {
                                                    _ = weak_inc.update(cx, |this, cx| {
                                                        this.config.editor.font_size =
                                                            (this.config.editor.font_size + 1.0)
                                                                .min(MAX_EDITOR_FONT_SIZE);
                                                        this.persist_config(cx);
                                                        cx.notify();
                                                    });
                                                }),
                                        ),
                                ),
                        ),
                )
                .footer(
                    DialogFooter::new().child(
                        Button::new("config-done")
                            .secondary()
                            .bg(black())
                            .label("Done")
                            .on_click(|_, window, cx| {
                                window.close_dialog(cx);
                            }),
                    ),
                )
        });
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
            .child(self.render_body(cx))
            .child(self.render_foot(cx))
    }
}
