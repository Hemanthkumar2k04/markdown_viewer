use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

const MAX_RECENT: usize = 10;

static TMP_SEQ: AtomicU64 = AtomicU64::new(1);
static SAVE_STATE: Mutex<u64> = Mutex::new(0);

#[derive(Default, Clone)]
pub(crate) struct RecentFiles {
    pub(crate) entries: VecDeque<PathBuf>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct RecentFilesFile {
    #[serde(default)]
    version: u32,
    #[serde(default)]
    files: Vec<PathBuf>,
}

impl RecentFiles {
    pub(crate) fn touch(&mut self, path: PathBuf) {
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

    pub(crate) fn remove(&mut self, path: &Path) {
        self.entries.retain(|p| p != path);
    }

    pub(crate) fn load(path: &Path) -> Self {
        let Ok(text) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        let Ok(file) = serde_json::from_str::<RecentFilesFile>(&text) else {
            return Self::default();
        };
        if file.version > 1 {
            return Self::default();
        }
        let mut recent = Self {
            entries: file.files.into(),
        };
        recent.remove_missing();
        recent
    }

    pub(crate) fn save_seq(&self, path: &Path, seq: u64) -> std::io::Result<()> {
        let mut last_seq = SAVE_STATE.lock().unwrap_or_else(|e| e.into_inner());
        if seq < *last_seq {
            return Ok(());
        }

        let file = RecentFilesFile {
            version: 1,
            files: self.entries.iter().cloned().collect(),
        };
        let json = serde_json::to_string_pretty(&file).map_err(std::io::Error::other)?;
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("recent_files.json");
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
