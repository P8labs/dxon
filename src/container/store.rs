use anyhow::{Context, Result};
use std::path::PathBuf;

use crate::container::meta::ContainerMeta;
use crate::error::DxonError;

pub struct ContainerStore {
    pub base_dir: PathBuf,
}

impl ContainerStore {
    pub fn new(base_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&base_dir).with_context(|| {
            format!(
                "cannot create containers directory: {}\n  check that the parent directory exists and is writable",
                base_dir.display()
            )
        })?;
        Ok(Self { base_dir })
    }

    pub fn container_dir(&self, name: &str) -> PathBuf {
        self.base_dir.join(name)
    }

    pub fn rootfs_dir(&self, name: &str) -> PathBuf {
        self.container_dir(name).join("rootfs")
    }

    pub fn meta_path(&self, name: &str) -> PathBuf {
        self.container_dir(name).join("meta.json")
    }

    pub fn exists(&self, name: &str) -> bool {
        self.container_dir(name).exists()
    }

    pub fn save_meta(&self, meta: &ContainerMeta) -> Result<()> {
        let path = self.meta_path(&meta.name);
        let json =
            serde_json::to_string_pretty(meta).context("failed to serialize container metadata")?;
        std::fs::write(&path, &json)
            .with_context(|| format!("cannot write metadata: {}", path.display()))?;
        Ok(())
    }

    pub fn load_meta(&self, name: &str) -> Result<ContainerMeta> {
        if !self.exists(name) {
            return Err(DxonError::ContainerNotFound(name.to_string()).into());
        }
        let path = self.meta_path(name);
        let json = std::fs::read_to_string(&path)
            .with_context(|| format!("cannot read metadata: {}", path.display()))?;
        let meta: ContainerMeta = serde_json::from_str(&json).with_context(|| {
            format!(
                "corrupt metadata for container '{name}': {}",
                path.display()
            )
        })?;
        Ok(meta)
    }

    pub fn list(&self) -> Result<Vec<ContainerMeta>> {
        let mut containers = Vec::new();
        for entry in std::fs::read_dir(&self.base_dir).with_context(|| {
            format!(
                "cannot read containers directory: {}\n  run 'dxon config show' to verify storage path",
                self.base_dir.display()
            )
        })? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                let name = path.file_name().unwrap().to_string_lossy().to_string();
                if let Ok(meta) = self.load_meta(&name) {
                    containers.push(meta);
                }
            }
        }
        containers.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        Ok(containers)
    }

    pub fn create_dirs(&self, name: &str) -> Result<()> {
        let rootfs = self.rootfs_dir(name);
        std::fs::create_dir_all(&rootfs).with_context(|| {
            format!(
                "cannot create container directory: {}\n  check that {} is writable",
                rootfs.display(),
                self.base_dir.display()
            )
        })?;
        Ok(())
    }

    pub fn remove(&self, name: &str) -> Result<()> {
        let dir = self.container_dir(name);
        if !dir.exists() {
            return Err(DxonError::ContainerNotFound(name.to_string()).into());
        }

        if std::fs::remove_dir_all(&dir).is_ok() {
            return Ok(());
        }

        let status = crate::user::privileged_command("rm")
            .args(["-rf", "--", dir.to_str().unwrap()])
            .status()
            .with_context(|| {
                format!(
                    "cannot remove container directory: {}\n  check that you have write permission",
                    dir.display()
                )
            })?;
        if !status.success() {
            anyhow::bail!(
                "cannot remove container directory: {}\n  check that you have write permission",
                dir.display()
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container::meta::ContainerMeta;
    use tempfile::tempdir;

    #[test]
    fn new_creates_base_directory() {
        let dir = tempdir().unwrap();
        let base = dir.path().join("containers");
        ContainerStore::new(base.clone()).unwrap();
        assert!(base.exists());
    }

    #[test]
    fn exists_returns_false_for_unknown_container() {
        let dir = tempdir().unwrap();
        let store = ContainerStore::new(dir.path().to_path_buf()).unwrap();
        assert!(!store.exists("nonexistent"));
    }

    #[test]
    fn create_dirs_makes_container_and_rootfs() {
        let dir = tempdir().unwrap();
        let store = ContainerStore::new(dir.path().to_path_buf()).unwrap();
        store.create_dirs("mybox").unwrap();
        assert!(store.exists("mybox"));
        assert!(store.rootfs_dir("mybox").exists());
    }

    #[test]
    fn save_and_load_meta_roundtrip() {
        let dir = tempdir().unwrap();
        let store = ContainerStore::new(dir.path().to_path_buf()).unwrap();
        store.create_dirs("test").unwrap();
        let mut meta = ContainerMeta::new("test", "arch", "/tmp/rootfs");
        meta.template = Some("nodejs".into());
        meta.packages = vec!["git".into(), "curl".into()];
        meta.repo = Some("https://github.com/example/repo".into());
        store.save_meta(&meta).unwrap();

        let loaded = store.load_meta("test").unwrap();
        assert_eq!(loaded.name, "test");
        assert_eq!(loaded.distro, "arch");
        assert_eq!(loaded.template, Some("nodejs".into()));
        assert_eq!(loaded.packages, vec!["git", "curl"]);
        assert_eq!(loaded.repo, Some("https://github.com/example/repo".into()));
    }

    #[test]
    fn load_meta_on_nonexistent_container_returns_not_found() {
        let dir = tempdir().unwrap();
        let store = ContainerStore::new(dir.path().to_path_buf()).unwrap();
        let err = store.load_meta("ghost").unwrap_err();
        assert!(err.to_string().contains("ghost"));
    }

    #[test]
    fn list_returns_empty_for_empty_store() {
        let dir = tempdir().unwrap();
        let store = ContainerStore::new(dir.path().to_path_buf()).unwrap();
        let containers = store.list().unwrap();
        assert!(containers.is_empty());
    }

    #[test]
    fn list_is_sorted_by_created_at_ascending() {
        let dir = tempdir().unwrap();
        let store = ContainerStore::new(dir.path().to_path_buf()).unwrap();
        for name in &["first", "second", "third"] {
            store.create_dirs(name).unwrap();
            let meta = ContainerMeta::new(name, "arch", "/tmp/rootfs");
            store.save_meta(&meta).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let containers = store.list().unwrap();
        assert_eq!(containers.len(), 3);
        assert!(containers[0].created_at <= containers[1].created_at);
        assert!(containers[1].created_at <= containers[2].created_at);
        assert_eq!(containers[0].name, "first");
        assert_eq!(containers[1].name, "second");
        assert_eq!(containers[2].name, "third");
    }

    #[test]
    fn remove_deletes_container_directory() {
        let dir = tempdir().unwrap();
        let store = ContainerStore::new(dir.path().to_path_buf()).unwrap();
        store.create_dirs("toremove").unwrap();
        let meta = ContainerMeta::new("toremove", "arch", "/tmp/r");
        store.save_meta(&meta).unwrap();
        assert!(store.exists("toremove"));
        store.remove("toremove").unwrap();
        assert!(!store.exists("toremove"));
    }

    #[test]
    fn remove_nonexistent_returns_container_not_found() {
        let dir = tempdir().unwrap();
        let store = ContainerStore::new(dir.path().to_path_buf()).unwrap();
        let err = store.remove("ghost").unwrap_err();
        assert!(err.to_string().contains("ghost"));
    }

    #[test]
    fn list_skips_dirs_without_meta_json() {
        let dir = tempdir().unwrap();
        let store = ContainerStore::new(dir.path().to_path_buf()).unwrap();
        // dir with no meta.json — simulates a partial/interrupted create
        std::fs::create_dir_all(store.container_dir("broken")).unwrap();
        store.create_dirs("valid").unwrap();
        let meta = ContainerMeta::new("valid", "debian", "/tmp/rootfs");
        store.save_meta(&meta).unwrap();
        let containers = store.list().unwrap();
        assert_eq!(containers.len(), 1);
        assert_eq!(containers[0].name, "valid");
    }

    #[test]
    fn list_skips_non_directory_entries_in_base_dir() {
        let dir = tempdir().unwrap();
        let store = ContainerStore::new(dir.path().to_path_buf()).unwrap();
        std::fs::write(store.base_dir.join("stray_file.txt"), b"data").unwrap();
        let containers = store.list().unwrap();
        assert!(containers.is_empty());
    }

    #[test]
    fn corrupt_meta_json_returns_error() {
        let dir = tempdir().unwrap();
        let store = ContainerStore::new(dir.path().to_path_buf()).unwrap();
        store.create_dirs("bad").unwrap();
        std::fs::write(store.meta_path("bad"), b"not valid json {{{{").unwrap();
        let err = store.load_meta("bad").unwrap_err();
        assert!(err.to_string().contains("bad"));
    }

    #[test]
    fn container_name_with_hyphens_and_numbers() {
        let dir = tempdir().unwrap();
        let store = ContainerStore::new(dir.path().to_path_buf()).unwrap();
        store.create_dirs("my-container-42").unwrap();
        let meta = ContainerMeta::new("my-container-42", "alpine", "/tmp/r");
        store.save_meta(&meta).unwrap();
        let loaded = store.load_meta("my-container-42").unwrap();
        assert_eq!(loaded.name, "my-container-42");
        assert_eq!(loaded.distro, "alpine");
    }

    #[test]
    fn duplicate_save_meta_overwrites_previous() {
        let dir = tempdir().unwrap();
        let store = ContainerStore::new(dir.path().to_path_buf()).unwrap();
        store.create_dirs("box").unwrap();
        let mut meta = ContainerMeta::new("box", "arch", "/tmp/r");
        store.save_meta(&meta).unwrap();
        meta.packages = vec!["vim".into()];
        store.save_meta(&meta).unwrap();
        let loaded = store.load_meta("box").unwrap();
        assert_eq!(loaded.packages, vec!["vim"]);
    }
}
