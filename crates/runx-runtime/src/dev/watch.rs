use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use thiserror::Error;

pub const DEFAULT_DEV_WATCH_DEBOUNCE_MS: u64 = 120;

const IGNORED_NAMES: &[&str] = &[
    ".git",
    ".hg",
    ".svn",
    "node_modules",
    "target",
    "dist",
    "build",
    "coverage",
    ".DS_Store",
];

const IGNORED_SUFFIXES: &[&str] = &[".tmp", ".swp", ".swo", "~"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DevWatchOptions {
    pub root: PathBuf,
    pub debounce: Duration,
    pub extra_ignored_names: Vec<String>,
}

impl DevWatchOptions {
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            debounce: Duration::from_millis(DEFAULT_DEV_WATCH_DEBOUNCE_MS),
            extra_ignored_names: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DevWatchSnapshot {
    files: BTreeMap<PathBuf, WatchedFileState>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DevWatchEvent {
    pub path: PathBuf,
    pub kind: DevWatchEventKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DevWatchEventKind {
    Created,
    Modified,
    Removed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DevWatchTrigger {
    pub events: Vec<DevWatchEvent>,
}

#[derive(Clone, Debug)]
pub struct PollingDevWatcher {
    options: DevWatchOptions,
    snapshot: DevWatchSnapshot,
    pending: Vec<DevWatchEvent>,
    last_event_at: Option<Instant>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct WatchedFileState {
    modified: Option<SystemTime>,
    len: u64,
}

#[derive(Debug, Error)]
pub enum DevWatchError {
    #[error("failed to scan watch root {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

impl PollingDevWatcher {
    pub fn new(options: DevWatchOptions) -> Result<Self, DevWatchError> {
        let snapshot = collect_watch_snapshot(&options)?;
        Ok(Self {
            options,
            snapshot,
            pending: Vec::new(),
            last_event_at: None,
        })
    }

    pub fn poll(&mut self) -> Result<Option<DevWatchTrigger>, DevWatchError> {
        let next = collect_watch_snapshot(&self.options)?;
        let events = diff_snapshots(&self.snapshot, &next);
        self.snapshot = next;
        if !events.is_empty() {
            self.pending.extend(events);
            self.last_event_at = Some(Instant::now());
            return Ok(None);
        }
        if self.pending.is_empty() {
            return Ok(None);
        }
        let Some(last_event_at) = self.last_event_at else {
            return Ok(None);
        };
        if last_event_at.elapsed() < self.options.debounce {
            return Ok(None);
        }
        let mut events = Vec::new();
        std::mem::swap(&mut events, &mut self.pending);
        self.last_event_at = None;
        Ok(Some(DevWatchTrigger { events }))
    }
}

pub fn collect_watch_snapshot(
    options: &DevWatchOptions,
) -> Result<DevWatchSnapshot, DevWatchError> {
    let mut files = BTreeMap::new();
    collect_watch_snapshot_inner(&options.root, options, &mut files)?;
    Ok(DevWatchSnapshot { files })
}

#[must_use]
pub fn should_ignore_dev_watch_path(path: &Path, extra_ignored_names: &[String]) -> bool {
    path.components().any(|component| {
        let name = component.as_os_str().to_string_lossy();
        IGNORED_NAMES.iter().any(|ignored| name == *ignored)
            || extra_ignored_names
                .iter()
                .any(|ignored| name.as_ref() == ignored)
            || IGNORED_SUFFIXES.iter().any(|suffix| name.ends_with(suffix))
            || (name == ".runx"
                && path
                    .components()
                    .any(|nested| nested.as_os_str() == "receipts"))
    })
}

fn collect_watch_snapshot_inner(
    directory: &Path,
    options: &DevWatchOptions,
    files: &mut BTreeMap<PathBuf, WatchedFileState>,
) -> Result<(), DevWatchError> {
    if should_ignore_dev_watch_path(directory, &options.extra_ignored_names) {
        return Ok(());
    }
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(source) => {
            return Err(DevWatchError::Io {
                path: directory.to_path_buf(),
                source,
            });
        }
    };
    for entry in entries {
        let entry = entry.map_err(|source| DevWatchError::Io {
            path: directory.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        if should_ignore_dev_watch_path(&path, &options.extra_ignored_names) {
            continue;
        }
        let metadata = entry.metadata().map_err(|source| DevWatchError::Io {
            path: path.clone(),
            source,
        })?;
        if metadata.is_dir() {
            collect_watch_snapshot_inner(&path, options, files)?;
        } else if metadata.is_file() {
            files.insert(
                path,
                WatchedFileState {
                    modified: metadata.modified().ok(),
                    len: metadata.len(),
                },
            );
        }
    }
    Ok(())
}

fn diff_snapshots(left: &DevWatchSnapshot, right: &DevWatchSnapshot) -> Vec<DevWatchEvent> {
    let mut events = Vec::new();
    for (path, state) in &right.files {
        match left.files.get(path) {
            None => events.push(DevWatchEvent {
                path: path.clone(),
                kind: DevWatchEventKind::Created,
            }),
            Some(previous) if previous != state => events.push(DevWatchEvent {
                path: path.clone(),
                kind: DevWatchEventKind::Modified,
            }),
            Some(_) => {}
        }
    }
    for path in left.files.keys() {
        if !right.files.contains_key(path) {
            events.push(DevWatchEvent {
                path: path.clone(),
                kind: DevWatchEventKind::Removed,
            });
        }
    }
    events
}
