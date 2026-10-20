use colori_core::game_log::StructuredGameLog;
use regex::Regex;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

pub struct TaggedGameLog {
    pub log: StructuredGameLog,
    pub batch_id: String,
}

impl Clone for TaggedGameLog {
    fn clone(&self) -> Self {
        TaggedGameLog {
            log: self.log.clone(),
            batch_id: self.batch_id.clone(),
        }
    }
}

pub enum LoadResult {
    Idle,
    Loading,
    Done(Vec<TaggedGameLog>),
    Error(String),
}

pub struct LogLoader {
    receiver: Option<mpsc::Receiver<Result<Vec<TaggedGameLog>, String>>>,
    is_loading: bool,
}

impl LogLoader {
    pub fn new() -> Self {
        LogLoader {
            receiver: None,
            is_loading: false,
        }
    }

    pub fn start_loading(&mut self, dir: &Path) {
        let dir = dir.to_path_buf();
        let (tx, rx) = mpsc::channel();
        self.receiver = Some(rx);
        self.is_loading = true;

        std::thread::spawn(move || {
            let result = load_logs_from_dir(&dir);
            let _ = tx.send(result);
        });
    }

    pub fn is_loading(&self) -> bool {
        self.is_loading
    }

    /// Non-blocking poll. Returns Done/Error only once (takes ownership).
    pub fn poll(&mut self) -> LoadResult {
        if let Some(ref receiver) = self.receiver {
            match receiver.try_recv() {
                Ok(Ok(logs)) => {
                    self.receiver = None;
                    self.is_loading = false;
                    LoadResult::Done(logs)
                }
                Ok(Err(e)) => {
                    self.receiver = None;
                    self.is_loading = false;
                    LoadResult::Error(e)
                }
                Err(mpsc::TryRecvError::Empty) => LoadResult::Loading,
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.receiver = None;
                    self.is_loading = false;
                    LoadResult::Error("Log loading thread disconnected unexpectedly".to_string())
                }
            }
        } else {
            LoadResult::Idle
        }
    }
}

/// Every `game-*.json` under `dir`, subdirectories included, oldest first: the
/// names start with a timestamp.
pub fn find_game_log_files(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    collect_game_log_files(dir, &mut files)?;
    files.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
    Ok(files)
}

fn collect_game_log_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let read_dir = std::fs::read_dir(dir).map_err(|e| format!("Failed to read directory {:?}: {}", dir, e))?;

    for entry in read_dir {
        let entry = entry.map_err(|e| format!("Failed to read dir entry: {}", e))?;
        let path = entry.path();

        // A symlinked directory is not followed, so a link back up the tree cannot loop.
        if entry.file_type().is_ok_and(|t| t.is_dir()) {
            collect_game_log_files(&path, files)?;
            continue;
        }

        let is_game_log = path.is_file()
            && path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|name| name.starts_with("game-") && name.ends_with(".json"));
        if is_game_log {
            files.push(path);
        }
    }
    Ok(())
}

fn load_logs_from_dir(dir: &Path) -> Result<Vec<TaggedGameLog>, String> {
    let batch_re =
        Regex::new(r"game-\d+-([a-z0-9]{6})(?:-[a-z0-9]{4})?\.json").map_err(|e| format!("Regex error: {}", e))?;

    let files = find_game_log_files(dir)?;
    let mut logs = Vec::with_capacity(files.len());

    for path in files {
        let batch_id = path
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(|name| batch_re.captures(name))
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();

        let contents = std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read {:?}: {}", path, e))?;
        let log: StructuredGameLog = serde_json::from_str(&contents)
            .map_err(|e| format!("Failed to parse {:?}: {}", path, e))?;
        logs.push(TaggedGameLog { log, batch_id });
    }

    Ok(logs)
}

#[cfg(test)]
mod tests {
    use super::find_game_log_files;
    use std::path::PathBuf;

    #[test]
    fn game_logs_are_found_in_subdirectories_oldest_first() {
        let root = std::env::temp_dir().join(format!("colori-log-loader-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("a/b")).unwrap();
        for file in [
            "game-300-aaaaaa-1111.json",
            "a/game-100-bbbbbb-2222.json",
            "a/b/game-200-cccccc-3333.json",
            "a/notes.json",
            "a/game-400-dddddd-4444.txt",
        ] {
            std::fs::write(root.join(file), "{}").unwrap();
        }

        let found: Vec<PathBuf> = find_game_log_files(&root).unwrap();
        std::fs::remove_dir_all(&root).unwrap();

        let names: Vec<_> = found.iter().map(|p| p.file_name().unwrap().to_str().unwrap()).collect();
        assert_eq!(
            names,
            ["game-100-bbbbbb-2222.json", "game-200-cccccc-3333.json", "game-300-aaaaaa-1111.json"]
        );
    }
}
