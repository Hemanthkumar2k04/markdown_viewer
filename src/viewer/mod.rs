mod io;
mod view;

use std::path::PathBuf;

use gpui_kit::base::input::{EditorState, InputEvent};
use gpui_kit::*;

use crate::config::AppConfig;
use crate::recent_files::RecentFiles;

actions!(markdown_viewer, [Save]);

pub(crate) struct MarkdownViewer {
    file_path: Option<PathBuf>,
    content: Option<SharedString>,
    editor: Entity<EditorState>,
    _editor_subscription: Subscription,
    edit_generation: u64,
    file_generation: u64,
    dirty: bool,
    history: RecentFiles,
    recent_files_path: Option<PathBuf>,
    history_save_gen: u64,
    config: AppConfig,
    config_path: Option<PathBuf>,
    config_save_gen: u64,
    font_select_subscription: Option<Subscription>,
}

impl MarkdownViewer {
    pub(crate) fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        recent_files_path: Option<PathBuf>,
        config_path: Option<PathBuf>,
    ) -> Self {
        let editor = cx.new(|cx| EditorState::new(window, cx));
        let _editor_subscription = cx.subscribe(&editor, |this, editor, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                this.edit_generation = this.edit_generation.wrapping_add(1);
                let generation = this.edit_generation;
                let file_generation = this.file_generation;
                let editor = editor.clone();
                cx.spawn(async move |this, cx| {
                    cx.background_executor()
                        .timer(std::time::Duration::from_millis(150))
                        .await;
                    _ = this.update(cx, |this, cx| {
                        // Only the latest keystroke for the active document wins.
                        if this.file_generation == file_generation
                            && this.edit_generation == generation
                            && this.file_path.is_some()
                        {
                            this.content = Some(editor.read(cx).value());
                            this.dirty = true;
                            cx.notify();
                        }
                    });
                })
                .detach();
            }
        });
        let history = recent_files_path
            .as_deref()
            .map(RecentFiles::load)
            .unwrap_or_default();
        let config = config_path
            .as_deref()
            .map(AppConfig::load)
            .unwrap_or_default();
        Self {
            content: None,
            editor,
            _editor_subscription,
            edit_generation: 0,
            file_generation: 0,
            dirty: false,
            file_path: None,
            history,
            recent_files_path,
            history_save_gen: 0,
            config,
            config_path,
            config_save_gen: 0,
            font_select_subscription: None,
        }
    }
}
