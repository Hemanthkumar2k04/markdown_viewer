use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use gpui_kit::component::WindowExt as _;
use gpui_kit::*;

use super::MarkdownViewer;

const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;
static SAVE_TMP_SEQ: AtomicU64 = AtomicU64::new(1);

fn read_markdown_bounded(path: &Path) -> std::io::Result<String> {
    let meta = std::fs::metadata(path)?;
    if !meta.is_file() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Selected path is not a regular file",
        ));
    }
    if meta.len() > MAX_FILE_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "File exceeds the 16 MB size limit",
        ));
    }

    let file = std::fs::File::open(path)?;
    let mut buf = Vec::with_capacity(meta.len() as usize);
    file.take(MAX_FILE_BYTES + 1).read_to_end(&mut buf)?;
    if buf.len() as u64 > MAX_FILE_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "File exceeds the 16 MB size limit",
        ));
    }

    String::from_utf8(buf).map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))
}

fn write_file_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let target = match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => std::fs::canonicalize(path)?,
        _ => path.to_path_buf(),
    };
    let existing_perms = std::fs::metadata(&target).ok().map(|m| m.permissions());

    let parent = target.parent().unwrap_or_else(|| Path::new("."));
    let file_name = target
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("document.md");
    let seq = SAVE_TMP_SEQ.fetch_add(1, Ordering::Relaxed);
    let tmp = parent.join(format!(".{file_name}.{}.{seq}.tmp", std::process::id()));

    let write_result = (|| -> std::io::Result<()> {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)?;
        if let Some(perms) = existing_perms {
            let _ = file.set_permissions(perms);
        }
        std::io::Write::write_all(&mut file, bytes)?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&tmp, &target)?;
        Ok(())
    })();

    if write_result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    write_result
}

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
                _ = this.update_in(cx, |this, window, cx| {
                    this.show_error(format!("Failed to open file dialog: {err}"), window, cx);
                });
            }
            Err(_) => {}
        })
        .detach();
    }

    pub(super) fn load_file(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        self.file_generation = self.file_generation.wrapping_add(1);
        let load_gen = self.file_generation;
        let editor = self.editor.clone();
        let read_path = path.clone();

        cx.spawn_in(window, async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { read_markdown_bounded(&read_path) })
                .await;
            _ = this.update_in(cx, |this, window, cx| {
                if this.file_generation != load_gen {
                    return;
                }
                match result {
                    Ok(text) => {
                        let text: SharedString = text.into();
                        this.file_path = Some(path.clone());
                        this.dirty = false;
                        this.edit_generation = this.edit_generation.wrapping_add(1);
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
        if self.content.is_none() {
            return;
        }
        // Read directly from the editor so keystrokes within the 150 ms preview debounce are never lost,
        // and advance edit_generation so an in-flight pre-save debounce timer cannot re-dirty the file.
        let text = self.editor.read(cx).value();
        self.content = Some(text.clone());
        self.edit_generation = self.edit_generation.wrapping_add(1);
        let save_file_gen = self.file_generation;
        let save_edit_gen = self.edit_generation;

        cx.spawn_in(window, async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { write_file_atomic(&path, text.as_bytes()) })
                .await;

            _ = this.update_in(cx, |this, window, cx| {
                match result {
                    Ok(()) => {
                        if this.file_generation == save_file_gen
                            && this.edit_generation == save_edit_gen
                        {
                            this.dirty = false;
                        }
                    }
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
        let Some(recent_files_path) = self.recent_files_path.clone() else {
            return;
        };
        self.history_save_gen = self.history_save_gen.wrapping_add(1);
        let seq = self.history_save_gen;
        let snapshot = self.history.clone();

        cx.spawn_in(window, async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { snapshot.save_seq(&recent_files_path, seq) })
                .await;
            if let Err(err) = result {
                _ = this.update_in(cx, |this, window, cx| {
                    this.show_error(format!("Failed to save history: {err}"), window, cx);
                });
            }
        })
        .detach();
    }

    pub(super) fn persist_config(&mut self, cx: &mut Context<Self>) {
        let Some(config_path) = self.config_path.clone() else {
            return;
        };
        self.config_save_gen = self.config_save_gen.wrapping_add(1);
        let seq = self.config_save_gen;
        let snapshot = self.config.clone();

        cx.spawn(async move |_, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { snapshot.save_seq(&config_path, seq) })
                .await;
            if let Err(err) = result {
                eprintln!("Failed to save config: {err}");
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
