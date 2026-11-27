mod io;
mod view;

use std::path::PathBuf;

use gpui_kit::base::input::{EditorState, InputEvent};
use gpui_kit::*;

use crate::recent_files::RecentFiles;

actions!(markdown_viewer, [Save]);

pub(crate) struct MarkdownViewer {
    file_path: Option<PathBuf>,
    content: Option<SharedString>,
    editor: Entity<EditorState>,
    _editor_subscription: Subscription,
    edit_generation: u64,
    dirty: bool,
    history: RecentFiles,
    config_path: PathBuf,
}

impl MarkdownViewer {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>, config_path: PathBuf) -> Self {
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
}
