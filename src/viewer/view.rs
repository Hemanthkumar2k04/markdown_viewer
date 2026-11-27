use gpui_kit::base::StyledExt;
use gpui_kit::component::ThemeStyled;
use gpui_kit::component::input::Editor;
use gpui_kit::component::popover::Popover;
use gpui_kit::component::{
    ActiveTheme, Icon, IconName, Sizable, TitleBar,
    button::{Button, ButtonVariants},
    h_resizable,
    label::Label,
    resizable_panel,
    text::TextView,
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use super::{MarkdownViewer, Save};

const GEAR_ICON: &[u8] = include_bytes!("../icons/gear.svg");

impl MarkdownViewer {
    fn render_titlebar(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let weak_this = cx.weak_entity();

        div()
            .relative()
            .w_full()
            .child(
                TitleBar::new().child(
                    Popover::new("burger-menu")
                        .anchor(Anchor::TopLeft)
                        .p_1p5()
                        .trigger(
                            Button::new("burger")
                                .ghost()
                                .small()
                                .icon(IconName::Menu)
                                .tooltip("Menu"),
                        )
                        .content(move |_, _, cx| {
                            let weak_this = weak_this.clone();
                            let popover = cx.weak_entity();
                            let popover_opt = popover.clone();

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
                                            _ = popover
                                                .update(cx, |state, cx| state.dismiss(window, cx));
                                            _ = weak_this.update(cx, |this, cx| {
                                                this.pick_file(window, cx);
                                            });
                                        }),
                                )
                                .child(
                                    Button::new("menu-options")
                                        .ghost()
                                        .w_full()
                                        .child(
                                            div()
                                                .h_flex()
                                                .w_full()
                                                .items_center()
                                                .justify_start()
                                                .gap_2()
                                                .child(Icon::empty().data(GEAR_ICON).size_4())
                                                .child(Label::new("Options").text_base()),
                                        )
                                        .on_click(move |_, window, cx| {
                                            _ = popover_opt
                                                .update(cx, |state, cx| state.dismiss(window, cx));
                                        }),
                                )
                        }),
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
                    resizable_panel()
                        .size_range(px(200.)..Pixels::MAX) // optional, defaults to 100px min
                        .child(Editor::new(&self.editor).h(relative(1.)).bordered(false)),
                )
                .child(resizable_panel().size_range(px(200.)..Pixels::MAX).child(
                    div().size_full().p_4().child(
                        TextView::markdown("markdown-content", content.clone()).scrollable(true),
                    ),
                ))
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
                                self.history.entries.iter().cloned().map(|path| {
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
