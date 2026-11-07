//! Locations for project-specific IDE data.

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};

pub const DIR: &str = ".openidea";

fn locations() -> &'static RwLock<HashMap<PathBuf, PathBuf>> {
    static LOCATIONS: OnceLock<RwLock<HashMap<PathBuf, PathBuf>>> = OnceLock::new();
    LOCATIONS.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Registers a project before opening its services. Existing project-local data is
/// copied once, without removing or overwriting the original files.
pub fn register(config_dir: &Path, root: &Path) -> io::Result<PathBuf> {
    let root = dunce::canonicalize(root)?;
    let mut hash = 0xcbf29ce484222325u64;
    for b in root.as_os_str().as_encoded_bytes() {
        hash = (hash ^ u64::from(*b)).wrapping_mul(0x100000001b3);
    }
    let target = config_dir.join("projects").join(format!("{hash:016x}"));
    let mut locations = locations().write().unwrap_or_else(|e| e.into_inner());
    std::fs::create_dir_all(&target)?;
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(target.join(".migration.lock"))?;
    lock.lock()?;
    let identity = target.join("project-path.txt");
    let root_bytes = root.as_os_str().as_encoded_bytes();
    if identity.exists() {
        if std::fs::read(&identity)? != root_bytes {
            return Err(io::Error::new(io::ErrorKind::AlreadyExists, "project configuration key collision"));
        }
    } else {
        let staging = target.join(".project-path.tmp");
        std::fs::write(&staging, root_bytes)?;
        std::fs::rename(staging, &identity)?;
    }
    let marker = target.join(".migration-complete");
    let data = target.join("data");
    std::fs::create_dir_all(&data)?;
    if !marker.exists() {
        copy_legacy(&root.join(DIR), &data, &target.join(".migration-copy"), true)?;
        std::fs::write(marker, b"1")?;
    }
    locations.insert(root, data.clone());
    Ok(data)
}

fn copy_legacy(from: &Path, to: &Path, staging: &Path, top: bool) -> io::Result<()> {
    let entries = match std::fs::read_dir(from) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name();
        // Workspace manifests and development plugins remain at their documented locations.
        if top && matches!(name.to_str(), Some("workspace.json" | "plugins" | "bin" | ".gitignore")) {
            continue;
        }
        let kind = entry.file_type()?;
        let dest = to.join(name);
        if kind.is_symlink() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("cannot migrate symbolic link in project configuration: {}", entry.path().display()),
            ));
        } else if kind.is_dir() {
            std::fs::create_dir_all(&dest)?;
            copy_legacy(&entry.path(), &dest, staging, false)?;
        } else if kind.is_file() {
            if !dest.exists() {
                std::fs::copy(entry.path(), staging)?;
                std::fs::rename(staging, dest)?;
            }
        }
    }
    Ok(())
}

/// Registered projects use application configuration; standalone crate users retain
/// the legacy location until their host registers a project.
pub fn path(root: &Path, rel: impl AsRef<Path>) -> PathBuf {
    let locations = locations().read().unwrap_or_else(|e| e.into_inner());
    locations.get(root).cloned().unwrap_or_else(|| root.join(DIR)).join(rel)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn separates_projects_and_copies_legacy_once() {
        let tmp = tempfile::tempdir().unwrap();
        let a = tmp.path().join("a");
        let b = tmp.path().join("b");
        let config = tmp.path().join("config");
        std::fs::create_dir_all(a.join(DIR)).unwrap();
        std::fs::create_dir_all(&b).unwrap();
        std::fs::write(a.join(DIR).join("settings.toml"), "old").unwrap();
        std::fs::create_dir_all(a.join(DIR).join("shelf/change")).unwrap();
        std::fs::write(a.join(DIR).join("shelf/change/shelved.patch"), "patch").unwrap();
        std::fs::write(a.join(DIR).join("workspace.json"), "workspace").unwrap();
        let a = dunce::canonicalize(a).unwrap();
        let b = dunce::canonicalize(b).unwrap();
        let dest = register(&config, &a).unwrap();
        assert_eq!(std::fs::read_to_string(path(&a, "settings.toml")).unwrap(), "old");
        assert_eq!(std::fs::read_to_string(path(&a, "shelf/change/shelved.patch")).unwrap(), "patch");
        assert!(!path(&a, "workspace.json").exists());
        std::fs::write(path(&a, "settings.toml"), "new").unwrap();
        register(&config, &a).unwrap();
        assert_eq!(std::fs::read_to_string(dest.join("settings.toml")).unwrap(), "new");
        assert_eq!(std::fs::read_to_string(a.join(DIR).join("settings.toml")).unwrap(), "old");
        assert_ne!(dest, register(&config, &b).unwrap());
        assert!(!b.join(DIR).exists());
    }
}
