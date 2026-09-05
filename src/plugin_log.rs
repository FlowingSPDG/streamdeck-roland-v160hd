use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_LINES: usize = 200;

struct LogState {
    path: PathBuf,
    lines: Vec<String>,
}

static STATE: OnceLock<Mutex<LogState>> = OnceLock::new();

fn state() -> &'static Mutex<LogState> {
    STATE.get_or_init(|| {
        let path = default_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        Mutex::new(LogState {
            path,
            lines: Vec::new(),
        })
    })
}

fn default_path() -> PathBuf {
    let file = "com.flowingspdg.roland.v160hd.log";
    if cfg!(windows) {
        if let Ok(appdata) = std::env::var("APPDATA") {
            return PathBuf::from(appdata)
                .join("Elgato")
                .join("StreamDeck")
                .join("logs")
                .join(file);
        }
    }
    if cfg!(target_os = "macos") {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home)
                .join("Library")
                .join("Logs")
                .join("ElgatoStreamDeck")
                .join(file);
        }
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("streamdeck-roland-v160hd")
            .join(file);
    }
    std::env::temp_dir().join(file)
}

fn timestamp() -> String {
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}.{:03}", dur.as_secs(), dur.subsec_millis())
}

pub fn path() -> PathBuf {
    match state().lock() {
        Ok(s) => s.path.clone(),
        Err(e) => e.into_inner().path.clone(),
    }
}

pub fn path_display() -> String {
    path().to_string_lossy().into_owned()
}

pub fn write_line(message: &str) {
    let line = format!("{} {message}", timestamp());
    eprintln!("{line}");
    if let Ok(mut state) = state().lock() {
        append_file(&state.path, &line);
        state.lines.push(line);
        let extra = state.lines.len().saturating_sub(MAX_LINES);
        if extra > 0 {
            state.lines.drain(..extra);
        }
    }
}

pub fn tail() -> String {
    state()
        .lock()
        .map(|s| s.lines.join("\n"))
        .unwrap_or_default()
}

fn append_file(path: &Path, line: &str) {
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{line}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_path_uses_named_log_file() {
        assert!(default_path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .contains("com.flowingspdg.roland.v160hd.log"));
    }
}
