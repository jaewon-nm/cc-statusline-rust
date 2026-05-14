//! Atomic filesystem helpers shared across config save, install, and uninstall.
//!
//! Concurrent writers into the same directory must not collide on a fixed
//! `<path>.tmp` sentinel — two parallel installs from different shells would
//! race. We disambiguate with `pid` plus a process-local monotonic counter so
//! every `atomic_write_bytes` call against the same path picks a unique temp.

use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::error::{Error, Result};

/// Serialize and rename. The temp file lives in the same directory as the
/// destination so the rename stays atomic on a single filesystem; cross-volume
/// moves are not atomic on Windows and would defeat the purpose.
pub fn atomic_write_bytes(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path.parent().ok_or_else(|| Error::FileIo {
        operation: "atomic_write_parent",
        path: path.to_path_buf(),
        source: io_error_other("destination has no parent directory"),
    })?;
    if !parent.as_os_str().is_empty() {
        fs::create_dir_all(parent).map_err(|source| Error::FileIo {
            operation: "atomic_write_mkdir",
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let tmp = temp_sibling(path);
    fs::write(&tmp, bytes).map_err(|source| Error::FileIo {
        operation: "atomic_write_tmp",
        path: tmp.clone(),
        source,
    })?;
    fs::rename(&tmp, path).map_err(|source| Error::FileIo {
        operation: "atomic_write_rename",
        path: path.to_path_buf(),
        source,
    })?;
    Ok(())
}

fn temp_sibling(path: &Path) -> std::path::PathBuf {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let filename = path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "out".to_owned());
    let with_suffix = format!("{filename}.tmp.{pid}.{seq:06}");
    match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p.join(with_suffix),
        _ => std::path::PathBuf::from(with_suffix),
    }
}

fn io_error_other(msg: &'static str) -> std::io::Error {
    std::io::Error::other(msg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_bytes_and_replaces_existing() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("note.txt");
        fs::write(&target, b"old").unwrap();

        atomic_write_bytes(&target, b"new contents").unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"new contents");
    }

    #[test]
    fn creates_parent_dirs_when_missing() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("nested/deep/file.json");
        atomic_write_bytes(&target, b"hi").unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"hi");
    }

    #[test]
    fn temp_filename_contains_pid_and_unique_seq() {
        let path = std::path::Path::new("/tmp/example.json");
        let a = temp_sibling(path);
        let b = temp_sibling(path);
        assert_ne!(a, b, "consecutive temp names must differ");
        let a_name = a.file_name().unwrap().to_string_lossy().into_owned();
        let pid = std::process::id().to_string();
        assert!(a_name.contains(&pid), "temp name {a_name} lacks pid {pid}");
        assert!(a_name.starts_with("example.json.tmp."));
    }

    #[test]
    fn temp_filename_handles_pathless_target() {
        let path = std::path::Path::new("bare.txt");
        let t = temp_sibling(path);
        assert!(
            t.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("bare.txt.tmp."),
            "got {t:?}",
        );
    }

    #[test]
    fn seq_increments_within_process() {
        // Manipulating the static directly would fight other tests; use the
        // observable behavior: two adjacent calls produce distinct names that
        // share the same pid prefix.
        let a = temp_sibling(std::path::Path::new("/tmp/foo.json"));
        let b = temp_sibling(std::path::Path::new("/tmp/foo.json"));
        let a_name = a.file_name().unwrap().to_string_lossy().into_owned();
        let b_name = b.file_name().unwrap().to_string_lossy().into_owned();
        assert_ne!(a_name, b_name);
    }
}
