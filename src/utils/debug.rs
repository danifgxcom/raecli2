use crate::config::DebugConfig;
use std::fs;
use std::path::PathBuf;

pub struct DebugLogger {
    config: DebugConfig,
    log_buffer: String,
}

impl DebugLogger {
    pub fn new(config: DebugConfig) -> Self {
        Self {
            config,
            log_buffer: String::new(),
        }
    }

    pub fn info(&mut self, message: &str) {
        self.log("INFO", message);
    }

    pub fn debug(&mut self, message: &str) {
        if self.config.verbose_tls {
            self.log("DEBUG", message);
        }
    }

    pub fn error(&mut self, message: &str) {
        self.log("ERROR", message);
    }

    fn log(&mut self, level: &str, message: &str) {
        let log_line = format!("[{}] {}\n", level, message);

        if self.config.enabled {
            eprint!("{}", log_line);
        }

        if self.config.save_logs {
            self.log_buffer.push_str(&log_line);
        }
    }

    pub fn save_to_file(&self, filename: &str) -> Result<(), std::io::Error> {
        if !self.config.save_logs {
            return Ok(());
        }

        let path = get_debug_file_path(filename);
        fs::write(path, &self.log_buffer)
    }

    pub fn save_html(&self, content: &str, filename: &str) -> Result<(), std::io::Error> {
        if !self.config.save_html {
            return Ok(());
        }

        let path = get_debug_file_path(filename);
        fs::write(path, content)
    }
}

fn get_debug_file_path(filename: &str) -> PathBuf {
    match std::env::current_dir() {
        Ok(current_dir) => current_dir.join(filename),
        Err(_) => PathBuf::from(filename),
    }
}
