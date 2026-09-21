use chrono::Utc;
use serde_json::Value;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

pub const DEFAULT_RETENTION_DAYS: u64 = 7;

pub fn cleanup_old_logs(log_dir: &Path, max_age_days: u64) {
    if !log_dir.is_dir() {
        return;
    }
    let max_age = Duration::from_secs(max_age_days * 86400);
    let now = SystemTime::now();

    if let Ok(entries) = fs::read_dir(log_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == "jsonl") {
                if let Ok(metadata) = path.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        if let Ok(age) = now.duration_since(modified) {
                            if age > max_age {
                                let _ = fs::remove_file(&path);
                            }
                        }
                    }
                }
            }
        }
    }
}

pub struct GameLogger {
    enabled: bool,
    step: usize,
    file: Option<File>,
    path: Option<PathBuf>,
}

impl GameLogger {
    pub fn new(log_dir: &Path, character: &str, seed: &str, enabled: bool) -> Self {
        if !enabled {
            return Self {
                enabled: false,
                step: 0,
                file: None,
                path: None,
            };
        }

        let _ = fs::create_dir_all(log_dir);
        cleanup_old_logs(log_dir, DEFAULT_RETENTION_DAYS);

        let now = Utc::now();
        let ts = now.format("%Y%m%d_%H%M%S");
        let safe_seed = seed.replace(['/', '\\'], "_");
        let filename = format!("{}_{}_{}.jsonl", ts, character, safe_seed);
        let path = log_dir.join(filename);

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .ok();

        Self {
            enabled: file.is_some(),
            step: 0,
            file,
            path: Some(path),
        }
    }

    pub fn log_state(&mut self, state: &Value) {
        if !self.enabled {
            return;
        }
        self.step += 1;
        let entry = serde_json::json!({
            "step": self.step,
            "ts": Utc::now().to_rfc3339(),
            "type": "state",
            "data": state,
        });
        self.write_entry(&entry);
    }

    pub fn log_action(&mut self, action: &Value) {
        if !self.enabled {
            return;
        }
        let entry = serde_json::json!({
            "step": self.step,
            "ts": Utc::now().to_rfc3339(),
            "type": "action",
            "data": action,
        });
        self.write_entry(&entry);
    }

    fn write_entry(&mut self, entry: &Value) {
        if let Some(ref mut f) = self.file {
            if let Ok(line) = serde_json::to_string(entry) {
                let _ = writeln!(f, "{}", line);
                let _ = f.flush();
            }
        }
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }
}
