use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{new_debouncer, DebouncedEvent, Debouncer};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WatcherError {
    #[error("Watcher error: {0}")]
    Notify(#[from] notify::Error),
    #[error("Channel error")]
    Channel,
}

pub type WatcherResult<T> = Result<T, WatcherError>;

#[derive(Debug, Clone)]
pub enum FileEvent {
    Created(PathBuf),
    Modified(PathBuf),
    Deleted(PathBuf),
}

pub struct FileWatcher {
    _watcher: Debouncer<RecommendedWatcher>,
    receiver: Receiver<Result<Vec<DebouncedEvent>, notify::Error>>,
    watched_path: PathBuf,
}

impl FileWatcher {
    pub fn new(path: &Path) -> WatcherResult<Self> {
        let (tx, rx) = channel();
        
        let mut debouncer = new_debouncer(
            Duration::from_millis(500),
            tx,
        )?;
        
        debouncer.watcher().watch(path, RecursiveMode::Recursive)?;
        
        Ok(Self {
            _watcher: debouncer,
            receiver: rx,
            watched_path: path.to_path_buf(),
        })
    }

    pub fn try_recv_events(&self) -> Vec<FileEvent> {
        let mut events = Vec::new();
        
        while let Ok(result) = self.receiver.try_recv() {
            if let Ok(debounced_events) = result {
                for event in debounced_events {
                    if let Some(file_event) = self.process_event(event) {
                        events.push(file_event);
                    }
                }
            }
        }
        
        events
    }

    fn process_event(&self, event: DebouncedEvent) -> Option<FileEvent> {
        let path = event.path;
        
        // Only process markdown files
        if !is_markdown_file(&path) {
            return None;
        }
        
        // Check if file exists to determine event type
        if path.exists() {
            Some(FileEvent::Modified(path))
        } else {
            Some(FileEvent::Deleted(path))
        }
    }

    pub fn watched_path(&self) -> &Path {
        &self.watched_path
    }
}

fn is_markdown_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("md"))
        .unwrap_or(false)
}

/// Scan a directory for all markdown files
pub fn scan_directory(path: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let entry_path = entry.path();
            
            if entry_path.is_dir() {
                // Skip hidden directories
                if entry_path.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with('.'))
                    .unwrap_or(false)
                {
                    continue;
                }
                // Recursively scan subdirectories
                files.extend(scan_directory(&entry_path));
            } else if is_markdown_file(&entry_path) {
                files.push(entry_path);
            }
        }
    }
    
    files
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_markdown_file() {
        assert!(is_markdown_file(Path::new("test.md")));
        assert!(is_markdown_file(Path::new("test.MD")));
        assert!(!is_markdown_file(Path::new("test.txt")));
        assert!(!is_markdown_file(Path::new("test")));
    }
}

