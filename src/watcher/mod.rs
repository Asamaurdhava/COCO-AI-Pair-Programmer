pub mod monitor;

use anyhow::Result;
use tokio::sync::mpsc;
use std::path::Path;

use crate::app::FileEvent;

pub struct FileMonitor {
    inner: monitor::FileWatcher,
}

impl FileMonitor {
    pub async fn new(tx: mpsc::Sender<FileEvent>) -> Result<Self> {
        let watcher = monitor::FileWatcher::new(tx).await?;
        Ok(Self { inner: watcher })
    }

    pub async fn watch(&mut self, path: &Path) -> Result<()> {
        self.inner.watch(path).await
    }

    pub async fn unwatch(&mut self, path: &Path) -> Result<()> {
        self.inner.unwatch(path).await
    }

    pub async fn run(&mut self) -> Result<()> {
        self.inner.run().await
    }

    pub async fn stop(&mut self) -> Result<()> {
        self.inner.stop().await
    }
}

