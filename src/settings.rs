use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const MAX_RECENT: usize = 10;

/// Small per-user settings file in the platform config dir
/// (`~/.config/pqstudio/settings.json`,
/// `~/Library/Application Support/no.laerk.PQ-Studio/settings.json`,
/// `%APPDATA%\PQ Studio\settings.json`).
/// Unknown or missing fields fall back to defaults so older files still load.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub database: Option<PathBuf>,
    pub engine: Option<PathBuf>,
    pub recent_files: Vec<PathBuf>,
    pub window_size: Option<[f32; 2]>,
    pub last_dir: Option<PathBuf>,
}

pub fn home_dir() -> Option<PathBuf> {
    let var = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
    std::env::var_os(var).filter(|h| !h.is_empty()).map(PathBuf::from)
}

/// A Windows known folder from its environment variable (`APPDATA`,
/// `LOCALAPPDATA`), joined with the app's folder name.
fn windows_dir(var: &str) -> Option<PathBuf> {
    std::env::var_os(var)
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .map(|p| p.join("PQ Studio"))
}

/// `$XDG_CONFIG_HOME/pqstudio` (default `~/.config/pqstudio`) on Linux,
/// `~/Library/Application Support/no.laerk.PQ-Studio` on macOS,
/// `%APPDATA%\PQ Studio` on Windows.
pub fn config_dir() -> Option<PathBuf> {
    if cfg!(windows) {
        return windows_dir("APPDATA");
    }
    if cfg!(target_os = "macos") {
        return home_dir().map(|h| h.join("Library/Application Support/no.laerk.PQ-Studio"));
    }
    xdg_dir("XDG_CONFIG_HOME", ".config").map(|d| d.join("pqstudio"))
}

/// `$XDG_CACHE_HOME/pqstudio` (default `~/.cache/pqstudio`) on Linux,
/// `~/Library/Caches/no.laerk.PQ-Studio` on macOS,
/// `%LOCALAPPDATA%\PQ Studio\cache` on Windows.
pub fn cache_dir() -> Option<PathBuf> {
    if cfg!(windows) {
        return windows_dir("LOCALAPPDATA").map(|d| d.join("cache"));
    }
    if cfg!(target_os = "macos") {
        return home_dir().map(|h| h.join("Library/Caches/no.laerk.PQ-Studio"));
    }
    xdg_dir("XDG_CACHE_HOME", ".cache").map(|d| d.join("pqstudio"))
}

/// An XDG base directory: the variable if it holds an absolute path (as the
/// spec requires), else `~/<fallback>`.
fn xdg_dir(var: &str, fallback: &str) -> Option<PathBuf> {
    std::env::var_os(var)
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| home_dir().map(|h| h.join(fallback)))
}

impl Settings {
    pub fn file_path() -> Option<PathBuf> {
        config_dir().map(|d| d.join("settings.json"))
    }

    pub fn load() -> Self {
        Self::file_path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> std::io::Result<()> {
        let Some(path) = Self::file_path() else {
            return Ok(());
        };
        self.save_to(&path)
    }

    fn save_to(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(self)?)?;
        std::fs::rename(tmp, path)
    }

    pub fn add_recent(&mut self, path: &Path) {
        self.recent_files.retain(|p| p != path);
        self.recent_files.insert(0, path.to_path_buf());
        self.recent_files.truncate(MAX_RECENT);
        if let Some(dir) = path.parent() {
            self.last_dir = Some(dir.to_path_buf());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recent_files_dedupe_and_cap() {
        let mut s = Settings::default();
        for i in 0..12 {
            s.add_recent(Path::new(&format!("/tmp/f{i}.pqi")));
        }
        s.add_recent(Path::new("/tmp/f5.pqi"));
        assert_eq!(s.recent_files.len(), MAX_RECENT);
        assert_eq!(s.recent_files[0], PathBuf::from("/tmp/f5.pqi"));
        assert_eq!(s.recent_files.iter().filter(|p| p.ends_with("f5.pqi")).count(), 1);
        assert_eq!(s.last_dir, Some(PathBuf::from("/tmp")));
    }

    #[test]
    fn round_trips_and_tolerates_missing_fields() {
        let dir = std::env::temp_dir().join(format!("pq-studio-settings-{}", std::process::id()));
        let path = dir.join("settings.json");
        let mut s = Settings::default();
        s.database = Some(PathBuf::from("/x/pitzer.dat"));
        s.window_size = Some([1000.0, 700.0]);
        s.save_to(&path).unwrap();
        let back: Settings = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(back, s);
        let partial: Settings = serde_json::from_str(r#"{"database":"/y.dat"}"#).unwrap();
        assert_eq!(partial.database, Some(PathBuf::from("/y.dat")));
        assert!(partial.recent_files.is_empty());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
