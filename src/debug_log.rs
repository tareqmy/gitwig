use std::sync::{Mutex, OnceLock};

static LOGS: OnceLock<Mutex<Vec<String>>> = OnceLock::new();

pub fn log(level: &str, msg: &str) {
    let mutex = LOGS.get_or_init(|| Mutex::new(Vec::new()));
    if let Ok(mut guard) = mutex.lock() {
        if guard.len() >= 1000 {
            guard.remove(0);
        }
        let time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| {
                let secs = d.as_secs();
                format!(
                    "{:02}:{:02}:{:02}",
                    (secs / 3600) % 24,
                    (secs / 60) % 60,
                    secs % 60
                )
            })
            .unwrap_or_else(|_| "00:00:00".to_string());

        let log_msg = format!("[{}] [{}] {}", time, level, msg);
        guard.push(log_msg.clone());

        // Also write to ~/.gitwig/gitwig.log
        if let Some(home) = dirs::home_dir() {
            let log_dir = home.join(".gitwig");
            let _ = std::fs::create_dir_all(&log_dir);
            let log_path = log_dir.join("gitwig.log");
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_path)
            {
                use std::io::Write;
                let _ = writeln!(file, "{}", log_msg);
            }
        }
    }
}

pub fn info(msg: impl AsRef<str>) {
    log("INFO", msg.as_ref());
}

#[allow(dead_code)]
pub fn warn(msg: impl AsRef<str>) {
    log("WARN", msg.as_ref());
}

#[allow(dead_code)]
pub fn error(msg: impl AsRef<str>) {
    log("ERROR", msg.as_ref());
}

#[allow(dead_code)]
pub fn debug(msg: impl AsRef<str>) {
    log("DEBUG", msg.as_ref());
}

pub fn get_logs() -> Vec<String> {
    LOGS.get()
        .and_then(|m| m.lock().ok())
        .map(|guard| guard.clone())
        .unwrap_or_default()
}
