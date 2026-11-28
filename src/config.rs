use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

pub(crate) const DEFAULT_EDITOR_FONT_FAMILY: &str = "JetBrains Mono";
pub(crate) const DEFAULT_EDITOR_FONT_SIZE: f32 = 14.0;
pub(crate) const MIN_EDITOR_FONT_SIZE: f32 = 10.0;
pub(crate) const MAX_EDITOR_FONT_SIZE: f32 = 32.0;

static TMP_SEQ: AtomicU64 = AtomicU64::new(1);
static SAVE_STATE: Mutex<u64> = Mutex::new(0);

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct EditorConfig {
    #[serde(default = "default_editor_font_family")]
    pub(crate) font_family: String,
    #[serde(default = "default_editor_font_size")]
    pub(crate) font_size: f32,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            font_family: default_editor_font_family(),
            font_size: default_editor_font_size(),
        }
    }
}

fn default_editor_font_family() -> String {
    DEFAULT_EDITOR_FONT_FAMILY.to_string()
}

fn default_editor_font_size() -> f32 {
    DEFAULT_EDITOR_FONT_SIZE
}

#[derive(Default, Clone)]
pub(crate) struct AppConfig {
    pub(crate) editor: EditorConfig,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct ConfigFile {
    #[serde(default)]
    version: u32,
    #[serde(default)]
    editor: EditorConfig,
}

impl AppConfig {
    pub(crate) fn load(path: &Path) -> Self {
        let Ok(text) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        let Ok(mut file) = serde_json::from_str::<ConfigFile>(&text) else {
            return Self::default();
        };
        if file.version > 1 {
            return Self::default();
        }
        if file.editor.font_family.trim().is_empty() {
            file.editor.font_family = default_editor_font_family();
        }
        if !file.editor.font_size.is_finite() {
            file.editor.font_size = default_editor_font_size();
        }
        file.editor.font_size = file
            .editor
            .font_size
            .clamp(MIN_EDITOR_FONT_SIZE, MAX_EDITOR_FONT_SIZE);

        Self {
            editor: file.editor,
        }
    }

    pub(crate) fn save_seq(&self, path: &Path, seq: u64) -> std::io::Result<()> {
        let mut last_seq = SAVE_STATE.lock().unwrap_or_else(|e| e.into_inner());
        if seq < *last_seq {
            return Ok(());
        }

        let file = ConfigFile {
            version: 1,
            editor: self.editor.clone(),
        };
        let json = serde_json::to_string_pretty(&file).map_err(std::io::Error::other)?;
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("config.json");
        let tmp_seq = TMP_SEQ.fetch_add(1, Ordering::Relaxed);
        let tmp = parent.join(format!(".{file_name}.{}.{tmp_seq}.tmp", std::process::id()));

        if let Err(err) =
            std::fs::write(&tmp, json.as_bytes()).and_then(|()| std::fs::rename(&tmp, path))
        {
            let _ = std::fs::remove_file(&tmp);
            return Err(err);
        }

        *last_seq = seq;
        Ok(())
    }
}
