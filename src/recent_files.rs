use std::collections::VecDeque;
use std::path::{Path, PathBuf};

const MAX_RECENT: usize = 10;

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

    pub(crate) fn save(&self, path: &Path) -> std::io::Result<()> {
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
