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

fn load_logs_from_dir(dir: &Path) -> Result<Vec<TaggedGameLog>, String> {
    let batch_re =
        Regex::new(r"game-\d+-([a-z0-9]{6})(?:-[a-z0-9]{4})?\.json").map_err(|e| format!("Regex error: {}", e))?;

    let mut entries: Vec<(PathBuf, String)> = Vec::new();

    let read_dir =
        std::fs::read_dir(dir).map_err(|e| format!("Failed to read directory: {}", e))?;

    for entry in read_dir {
        let entry = entry.map_err(|e| format!("Failed to read dir entry: {}", e))?;
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        let file_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };

        if !file_name.starts_with("game-") || !file_name.ends_with(".json") {
            continue;
        }

        let batch_id = batch_re
            .captures(&file_name)
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();

        entries.push((path, batch_id));
    }

    entries.sort_by(|a, b| a.0.file_name().cmp(&b.0.file_name()));

    let mut logs = Vec::with_capacity(entries.len());

    for (path, batch_id) in entries {
        let contents = std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read {:?}: {}", path, e))?;
        let log: StructuredGameLog = serde_json::from_str(&contents)
            .map_err(|e| format!("Failed to parse {:?}: {}", path, e))?;
        logs.push(TaggedGameLog { log, batch_id });
    }

    Ok(logs)
}
