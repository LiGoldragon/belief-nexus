//! Selection of the one Belief memory file at a prepared location: a sole legacy file with the
//! sema extension is renamed to the current name; both present is a typed conflict.

use std::path::PathBuf;

#[derive(Debug)]
pub struct MemoryFiles {
    pub current: PathBuf,
    pub legacy: PathBuf,
}

pub trait MemoryFilesFromCurrent {
    fn from_current(current: PathBuf) -> Self;
}

pub trait MemoryFileSelecting {
    fn select(self) -> Result<PathBuf, MemoryFileSelectionError>;
}

#[derive(Debug)]
pub enum MemoryFileSelectionError {
    Inspection { path: PathBuf, source: std::io::Error },
    Conflict { current: PathBuf, legacy: PathBuf },
    Migration { current: PathBuf, legacy: PathBuf, source: std::io::Error },
}

impl MemoryFilesFromCurrent for MemoryFiles {
    fn from_current(current: PathBuf) -> Self {
        let legacy = current.with_extension("sema");
        Self { current, legacy }
    }
}

impl MemoryFileSelecting for MemoryFiles {
    fn select(self) -> Result<PathBuf, MemoryFileSelectionError> {
        let current_exists = self.current.try_exists().map_err(|source| MemoryFileSelectionError::Inspection {
            path: self.current.clone(),
            source,
        })?;
        let legacy_exists = self.legacy.try_exists().map_err(|source| MemoryFileSelectionError::Inspection {
            path: self.legacy.clone(),
            source,
        })?;
        match (current_exists, legacy_exists) {
            (true, true) => Err(MemoryFileSelectionError::Conflict { current: self.current, legacy: self.legacy }),
            (false, true) => {
                std::fs::rename(&self.legacy, &self.current).map_err(|source| MemoryFileSelectionError::Migration {
                    current: self.current.clone(),
                    legacy: self.legacy.clone(),
                    source,
                })?;
                Ok(self.current)
            }
            (true, false) | (false, false) => Ok(self.current),
        }
    }
}

impl std::fmt::Display for MemoryFileSelectionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Inspection { path, source } => write!(formatter, "could not inspect Memory path {}: {source}", path.display()),
            Self::Conflict { current, legacy } => write!(
                formatter,
                "Memory path conflict: current {} and legacy {} both exist",
                current.display(),
                legacy.display()
            ),
            Self::Migration { current, legacy, source } => write!(
                formatter,
                "could not migrate legacy Memory {} to current {}: {source}",
                legacy.display(),
                current.display()
            ),
        }
    }
}

impl std::error::Error for MemoryFileSelectionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Inspection { source, .. } | Self::Migration { source, .. } => Some(source),
            Self::Conflict { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{MemoryFileSelecting, MemoryFileSelectionError, MemoryFiles, MemoryFilesFromCurrent};

    trait MemoryFixture {
        fn memory_files(&self) -> MemoryFiles;
    }

    impl MemoryFixture for tempfile::TempDir {
        fn memory_files(&self) -> MemoryFiles {
            MemoryFiles::from_current(self.path().join("belief.memory"))
        }
    }

    #[test]
    fn legacy_only_memory_is_migrated_to_the_current_path() {
        let directory = tempfile::tempdir().unwrap();
        let files = directory.memory_files();
        std::fs::write(&files.legacy, b"memory").unwrap();
        let selected = files.select().unwrap();
        assert_eq!(selected, directory.path().join("belief.memory"));
        assert_eq!(std::fs::read(selected).unwrap(), b"memory");
        assert!(!directory.path().join("belief.sema").exists());
    }

    #[test]
    fn current_only_memory_is_used_without_change() {
        let directory = tempfile::tempdir().unwrap();
        let files = directory.memory_files();
        std::fs::write(&files.current, b"current").unwrap();
        let selected = files.select().unwrap();
        assert_eq!(std::fs::read(selected).unwrap(), b"current");
    }

    #[test]
    fn current_and_legacy_memory_are_a_typed_conflict_naming_both_paths() {
        let directory = tempfile::tempdir().unwrap();
        let files = directory.memory_files();
        std::fs::write(&files.current, b"current").unwrap();
        std::fs::write(&files.legacy, b"legacy").unwrap();
        let current = files.current.clone();
        let legacy = files.legacy.clone();
        let error = files.select().unwrap_err();
        match &error {
            MemoryFileSelectionError::Conflict { current: found_current, legacy: found_legacy } => {
                assert_eq!(found_current, &current);
                assert_eq!(found_legacy, &legacy);
            }
            other => panic!("expected typed Memory path conflict, got {other:?}"),
        }
        let message = error.to_string();
        assert!(message.contains(current.to_str().unwrap()));
        assert!(message.contains(legacy.to_str().unwrap()));
        assert_eq!(std::fs::read(current).unwrap(), b"current");
        assert_eq!(std::fs::read(legacy).unwrap(), b"legacy");
    }

    #[test]
    fn fresh_memory_selects_the_current_path_without_creating_it() {
        let directory = tempfile::tempdir().unwrap();
        let selected = directory.memory_files().select().unwrap();
        assert_eq!(selected, directory.path().join("belief.memory"));
        assert!(!selected.exists());
    }
}
