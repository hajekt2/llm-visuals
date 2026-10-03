//! Isolated mixed-host fixtures. No environment mutation or live GPU commands.
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

pub struct MixedHost {
    pub root: PathBuf,
}

impl MixedHost {
    pub fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "llm-visuals-mixed-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let fixture = Self { root };
        let files: std::collections::BTreeMap<String, String> =
            serde_json::from_str(include_str!("../fixtures/mixed-gpu/amd-sysfs.json")).unwrap();
        for (path, value) in files {
            fixture.write(Path::new("sys").join(path), value);
        }
        fixture
    }

    pub fn write(&self, path: impl AsRef<Path>, value: impl AsRef<[u8]>) {
        let path = self.root.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, value).unwrap();
    }
}

impl Drop for MixedHost {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
