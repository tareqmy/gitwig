//! Scratch-file fixtures shared by the unit tests of every module.
//!
//! `App::new` writes `keybindings.toml` (and, on a first run, `state.toml`,
//! `.version` and the config itself) beside the config path it is given. A
//! bare file name such as `"dummy.toml"` resolves against the process working
//! directory — the checkout, under `cargo test` — so a test must build its
//! `App` with [`temp_config_path`] instead.

use std::path::PathBuf;

/// Removes a scratch file when dropped.
pub(crate) struct TestFileGuard {
    pub(crate) path: PathBuf,
}

impl Drop for TestFileGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Removes a scratch directory, and everything in it, when dropped.
pub(crate) struct TestDirGuard {
    pub(crate) path: PathBuf,
}

impl Drop for TestDirGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// A `config.toml` path in a fresh scratch directory named after `tag` (unique
/// per test) and this process, so the files `App::new` writes beside it land
/// there. Bind the guard to a named variable (`_guard`, not `_`) so the
/// directory outlives the `App`.
pub(crate) fn temp_config_path(tag: &str) -> (PathBuf, TestDirGuard) {
    let dir = std::env::temp_dir().join(format!("gitwig_test_cfg_{}_{}", tag, std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch config dir should be creatable");
    (dir.join("config.toml"), TestDirGuard { path: dir })
}
