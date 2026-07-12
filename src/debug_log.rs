use std::sync::{Mutex, OnceLock};

static LOGS: OnceLock<Mutex<Vec<String>>> = OnceLock::new();

fn init_logs() -> Mutex<Vec<String>> {
    let mut initial_logs = Vec::new();
    if let Some(home) = dirs::home_dir() {
        let log_path = home.join(".gitwig").join("gitwig.log");
        if let Ok(file_content) = std::fs::read_to_string(&log_path) {
            for line in file_content.lines() {
                if !line.trim().is_empty() {
                    initial_logs.push(line.to_string());
                }
            }
            if initial_logs.len() > 1000 {
                initial_logs.drain(0..initial_logs.len() - 1000);
            }
        }
    }
    Mutex::new(initial_logs)
}

pub fn log(level: &str, msg: &str) {
    let mutex = LOGS.get_or_init(init_logs);
    if let Ok(mut guard) = mutex.lock() {
        if guard.len() >= 1000 {
            guard.remove(0);
        }
        let time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| {
                let secs = d.as_secs();
                format!("{:02}:{:02}:{:02}", (secs / 3600) % 24, (secs / 60) % 60, secs % 60)
            })
            .unwrap_or_else(|_| "00:00:00".to_string());

        let log_msg = format!("[{}] [{}] {}", time, level, msg);
        guard.push(log_msg.clone());

        // Also write to ~/.gitwig/gitwig.log
        if let Some(home) = dirs::home_dir() {
            let log_dir = home.join(".gitwig");
            let _ = std::fs::create_dir_all(&log_dir);
            let log_path = log_dir.join("gitwig.log");

            // Rotate log if it exceeds 5 MB
            if let Ok(metadata) = std::fs::metadata(&log_path) {
                if metadata.len() > 5 * 1024 * 1024 {
                    let backup_path = log_dir.join("gitwig.log.1");
                    let _ = std::fs::rename(&log_path, &backup_path);
                }
            }

            if let Ok(mut file) =
                std::fs::OpenOptions::new().create(true).append(true).open(&log_path)
            {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if let Ok(meta) = std::fs::metadata(&log_path) {
                        let mut perms = meta.permissions();
                        if perms.mode() & 0o777 != 0o600 {
                            perms.set_mode(0o600);
                            let _ = std::fs::set_permissions(&log_path, perms);
                        }
                    }
                }
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
    let mutex = LOGS.get_or_init(init_logs);
    if let Ok(guard) = mutex.lock() { guard.clone() } else { Vec::new() }
}

pub fn clear() {
    let mutex = LOGS.get_or_init(init_logs);
    if let Ok(mut guard) = mutex.lock() {
        guard.clear();
    }
    if let Some(home) = dirs::home_dir() {
        let log_path = home.join(".gitwig").join("gitwig.log");
        let _ = std::fs::write(&log_path, "");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_log_functions() {
        info("unique_info_msg_test");
        warn("unique_warn_msg_test");
        error("unique_error_msg_test");
        debug("unique_debug_msg_test");

        let logs = get_logs();
        assert!(logs.iter().any(|l| l.contains("[INFO] unique_info_msg_test")));
        assert!(logs.iter().any(|l| l.contains("[WARN] unique_warn_msg_test")));
        assert!(logs.iter().any(|l| l.contains("[ERROR] unique_error_msg_test")));
        assert!(logs.iter().any(|l| l.contains("[DEBUG] unique_debug_msg_test")));
        
        clear();
    }
}

