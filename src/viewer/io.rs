use std::path::PathBuf;

use gpui_kit::component::WindowExt as _;
use gpui_kit::*;

use super::MarkdownViewer;

impl MarkdownViewer {
    pub(super) fn pick_file(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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

    pub(super) fn load_file(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
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

    pub(super) fn save_file(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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

    pub(super) fn show_error(
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

    pub(super) fn persist_history(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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
    pub(super) fn remove_recent(
        &mut self,
        path: &std::path::Path,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.history.remove(path);
        self.persist_history(window, cx);
        cx.notify();
    }
}
