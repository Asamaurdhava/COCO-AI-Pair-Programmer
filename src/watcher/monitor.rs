use anyhow::{anyhow, Result};
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio::time::{Duration, Instant, sleep};
use chrono::Utc;

use crate::app::FileEvent;

pub struct FileWatcher {
    watcher: RecommendedWatcher,
    event_tx: mpsc::Sender<FileEvent>,
    watched_paths: Arc<Mutex<HashSet<PathBuf>>>,
    debounce_delay: Duration,
    last_events: Arc<Mutex<std::collections::HashMap<PathBuf, Instant>>>,
    running: Arc<Mutex<bool>>,
    _notify_rx: mpsc::Receiver<Event>,
}

impl FileWatcher {
    pub async fn new(event_tx: mpsc::Sender<FileEvent>) -> Result<Self> {
        let (notify_tx, notify_rx) = mpsc::channel(10);
        let watched_paths = Arc::new(Mutex::new(HashSet::new()));
        let last_events = Arc::new(Mutex::new(std::collections::HashMap::new()));
        let running = Arc::new(Mutex::new(false));

        // Create the file system watcher
        let notify_tx_clone = notify_tx.clone();
        let watcher = RecommendedWatcher::new(
            move |result: Result<Event, notify::Error>| {
                if let Ok(event) = result {
                    let _ = notify_tx_clone.blocking_send(event);
                }
            },
            Config::default(),
        ).map_err(|e| anyhow!("Failed to create file watcher: {}", e))?;

        Ok(Self {
            watcher,
            event_tx,
            watched_paths,
            debounce_delay: Duration::from_millis(300),
            last_events,
            running,
            _notify_rx: notify_rx,
        })
    }

    pub async fn watch(&mut self, path: &Path) -> Result<()> {
        tracing::info!("Starting to watch path: {}", path.display());

        self.watcher
            .watch(path, RecursiveMode::Recursive)
            .map_err(|e| anyhow!("Failed to watch path {}: {}", path.display(), e))?;

        self.watched_paths.lock().await.insert(path.to_path_buf());

        tracing::info!("Successfully watching path: {}", path.display());
        Ok(())
    }

    pub async fn unwatch(&mut self, path: &Path) -> Result<()> {
        tracing::info!("Stopping watch on path: {}", path.display());

        self.watcher
            .unwatch(path)
            .map_err(|e| anyhow!("Failed to unwatch path {}: {}", path.display(), e))?;

        self.watched_paths.lock().await.remove(path);

        tracing::info!("Successfully unwatched path: {}", path.display());
        Ok(())
    }

    pub async fn run(&mut self) -> Result<()> {
        tracing::info!("Starting file watcher event loop");
        *self.running.lock().await = true;

        // Take ownership of the receiver
        let mut notify_rx = std::mem::replace(&mut self._notify_rx, {
            let (_, rx) = mpsc::channel(1);
            rx
        });

        let event_tx = self.event_tx.clone();
        let last_events = self.last_events.clone();
        let debounce_delay = self.debounce_delay;
        let running = self.running.clone();

        // Spawn the event processing task
        let event_processor = tokio::spawn(async move {
            while *running.lock().await {
                tokio::select! {
                    event = notify_rx.recv() => {
                        if let Some(event) = event {
                            if let Err(e) = Self::process_notify_event(
                                event,
                                &event_tx,
                                &last_events,
                                debounce_delay
                            ).await {
                                tracing::error!("Error processing file event: {}", e);
                            }
                        } else {
                            tracing::debug!("Notify channel closed");
                            break;
                        }
                    }
                    _ = sleep(Duration::from_millis(100)) => {
                        // Periodic check to keep the loop alive
                    }
                }
            }
        });

        // Wait for the processor to complete
        event_processor.await.map_err(|e| anyhow!("Event processor task failed: {}", e))?;

        tracing::info!("File watcher event loop stopped");
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<()> {
        tracing::info!("Stopping file watcher");
        *self.running.lock().await = false;

        // Unwatch all paths
        let paths: Vec<_> = self.watched_paths.lock().await.clone().into_iter().collect();
        for path in paths {
            if let Err(e) = self.unwatch(&path).await {
                tracing::warn!("Failed to unwatch path {}: {}", path.display(), e);
            }
        }

        tracing::info!("File watcher stopped");
        Ok(())
    }

    async fn process_notify_event(
        event: Event,
        event_tx: &mpsc::Sender<FileEvent>,
        last_events: &Arc<Mutex<std::collections::HashMap<PathBuf, Instant>>>,
        debounce_delay: Duration,
    ) -> Result<()> {
        tracing::debug!("Processing notify event: {:?}", event);

        for path in &event.paths {
            // Check if we should process this file
            if !Self::should_process_file(path) {
                tracing::debug!("Skipping file: {}", path.display());
                continue;
            }

            // Debounce rapid file changes
            let now = Instant::now();
            {
                let mut last_events_map = last_events.lock().await;
                if let Some(&last_time) = last_events_map.get(path) {
                    if now.duration_since(last_time) < debounce_delay {
                        tracing::debug!("Debouncing file event for: {}", path.display());
                        continue;
                    }
                }
                last_events_map.insert(path.clone(), now);
            }

            // Read file content
            match Self::read_file_content(path).await {
                Ok(content) => {
                    let file_event = FileEvent {
                        path: path.clone(),
                        content,
                        event_type: event.kind,
                        timestamp: Utc::now(),
                    };

                    if let Err(e) = event_tx.send(file_event).await {
                        tracing::error!("Failed to send file event: {}", e);
                    } else {
                        tracing::debug!("Sent file event for: {}", path.display());
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to read file {}: {}", path.display(), e);
                }
            }
        }

        Ok(())
    }

    fn should_process_file(path: &Path) -> bool {
        // Skip hidden files and directories
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with('.') {
                return false;
            }
        }

        // Skip common build/cache directories
        let path_str = path.to_string_lossy().to_lowercase();
        let skip_patterns = [
            "target/",
            "node_modules/",
            ".git/",
            "build/",
            "dist/",
            "out/",
            "__pycache__/",
            ".pytest_cache/",
            ".vscode/",
            ".idea/",
        ];

        for pattern in &skip_patterns {
            if path_str.contains(pattern) {
                return false;
            }
        }

        // Skip temporary and backup files
        if let Some(extension) = path.extension().and_then(|e| e.to_str()) {
            let skip_extensions = [
                "tmp", "temp", "bak", "swp", "swo", "log",
                "lock", "pid", "pyc", "pyo", "class", "o",
                "so", "dylib", "dll", "exe", "min.js", "min.css",
            ];

            if skip_extensions.contains(&extension.to_lowercase().as_str()) {
                return false;
            }
        }

        // Check for supported file types
        if let Some(extension) = path.extension().and_then(|e| e.to_str()) {
            let supported_extensions = [
                "rs", "py", "js", "ts", "jsx", "tsx", "go", "java",
                "c", "cpp", "cc", "cxx", "h", "hpp", "cs", "rb",
                "php", "swift", "kt", "scala", "clj", "ex", "exs",
                "hs", "ml", "f", "f90", "lua", "r", "m", "mm",
                "dart", "elm", "nim", "zig", "v", "cr",
            ];

            return supported_extensions.contains(&extension.to_lowercase().as_str());
        }

        // Check for files without extensions that might be code
        if path.extension().is_none() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                let code_files = [
                    "Makefile", "Dockerfile", "Jenkinsfile", "Vagrantfile",
                    "Rakefile", "Gemfile", "Podfile", "CMakeLists.txt",
                ];

                return code_files.contains(&name);
            }
        }

        false
    }

    async fn read_file_content(path: &Path) -> Result<String> {
        // Check file size first to avoid reading huge files
        let metadata = tokio::fs::metadata(path).await
            .map_err(|e| anyhow!("Failed to read file metadata: {}", e))?;

        const MAX_FILE_SIZE: u64 = 8 * 1024; // 8KB
        if metadata.len() > MAX_FILE_SIZE {
            return Err(anyhow!("File too large: {} bytes", metadata.len()));
        }

        let content = tokio::fs::read_to_string(path).await
            .map_err(|e| anyhow!("Failed to read file content: {}", e))?;

        // Basic validation that this is likely text content
        if content.chars().take(1000).any(|c| c.is_control() && c != '\n' && c != '\r' && c != '\t') {
            return Err(anyhow!("File appears to contain binary data"));
        }

        Ok(content)
    }

    pub async fn get_watched_paths(&self) -> Vec<PathBuf> {
        self.watched_paths.lock().await.iter().cloned().collect()
    }

    pub async fn is_running(&self) -> bool {
        *self.running.lock().await
    }

    pub fn set_debounce_delay(&mut self, delay: Duration) {
        self.debounce_delay = delay;
    }
}

impl Drop for FileWatcher {
    fn drop(&mut self) {
        tracing::debug!("FileWatcher dropped");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_file_watcher_creation() {
        let (tx, _rx) = mpsc::channel(10);
        let watcher = FileWatcher::new(tx).await;
        assert!(watcher.is_ok());
    }

    #[tokio::test]
    async fn test_should_process_file() {
        assert!(FileWatcher::should_process_file(Path::new("test.rs")));
        assert!(FileWatcher::should_process_file(Path::new("src/main.py")));
        assert!(!FileWatcher::should_process_file(Path::new(".hidden")));
        assert!(!FileWatcher::should_process_file(Path::new("target/debug/app")));
        assert!(!FileWatcher::should_process_file(Path::new("file.tmp")));
    }

    #[tokio::test]
    async fn test_watch_unwatch() {
        let temp_dir = TempDir::new().unwrap();
        let (tx, _rx) = mpsc::channel(10);
        let mut watcher = FileWatcher::new(tx).await.unwrap();

        let result = watcher.watch(temp_dir.path()).await;
        assert!(result.is_ok());

        let paths = watcher.get_watched_paths().await;
        assert!(paths.contains(&temp_dir.path().to_path_buf()));

        let result = watcher.unwatch(temp_dir.path()).await;
        assert!(result.is_ok());

        let paths = watcher.get_watched_paths().await;
        assert!(!paths.contains(&temp_dir.path().to_path_buf()));
    }

    #[tokio::test]
    async fn test_read_file_content() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.rs");
        let content = "fn main() { println!(\"Hello, world!\"); }";

        fs::write(&file_path, content).unwrap();

        let result = FileWatcher::read_file_content(&file_path).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), content);
    }
}