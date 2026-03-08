/// End-to-end tests for the `dxon` binary.
///
/// Each test invokes the real binary against a fully-isolated temporary home
/// directory (`HOME`) and containers directory (`DXON_DIR`), so they never
/// touch the developer's real config or containers.
///
/// Tests that require external tools (`pacstrap`, `debootstrap`,
/// `systemd-nspawn`) or network access are marked `#[ignore]` and must be
/// opted into explicitly:
///
///   cargo test --test e2e                        # fast, offline tests
///   cargo test --test e2e -- --include-ignored   # all tests
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Test harness helpers
// ---------------------------------------------------------------------------

/// Locate the `dxon` binary built by Cargo.
///
/// Integration test binaries live in `target/debug/deps/`, so we pop up to
/// `target/debug/` and look for the main binary there.
fn dxon_bin() -> PathBuf {
    let mut path = std::env::current_exe()
        .expect("cannot determine current exe path")
        .canonicalize()
        .expect("cannot canonicalize exe path");

    // Remove the test binary name and the optional "deps" directory.
    path.pop();
    if path.file_name().map(|n| n == "deps").unwrap_or(false) {
        path.pop();
    }

    path.push("dxon");
    assert!(
        path.exists(),
        "dxon binary not found at {}\n  hint: run `cargo build` before `cargo test --test e2e`",
        path.display()
    );
    path
}

/// An isolated environment for a single test: a temporary directory used as
/// both `HOME` and the parent for a dedicated containers directory.
struct TestEnv {
    /// Temporary directory acting as the synthetic `HOME`.
    home: TempDir,
    /// Path used as the `DXON_DIR` (containers store) for this test.
    containers_dir: PathBuf,
}

impl TestEnv {
    fn new() -> Self {
        let home = TempDir::new().expect("failed to create temp dir");
        let containers_dir = home.path().join("containers");
        fs::create_dir_all(&containers_dir).expect("failed to create containers dir");
        TestEnv {
            home,
            containers_dir,
        }
    }

    /// Build a `Command` for `dxon` with the test environment pre-configured.
    ///
    /// Sets:
    ///  - `HOME`       → isolated temp directory (config lives here)
    ///  - `DXON_DIR`   → containers subdirectory inside the temp dir
    ///  - `NO_COLOR`   → disables ANSI escape codes for easy string matching
    ///  - clears `SUDO_USER` to avoid privilege escalation surprises
    fn cmd(&self, args: &[&str]) -> Command {
        let mut c = Command::new(dxon_bin());
        c.args(args)
            .env("HOME", self.home.path())
            .env("DXON_DIR", &self.containers_dir)
            .env("NO_COLOR", "1")
            .env_remove("SUDO_USER");
        c
    }

    /// Run `dxon <args>` and return the raw `Output`.
    fn run(&self, args: &[&str]) -> Output {
        self.cmd(args)
            .output()
            .expect("failed to execute dxon binary")
    }

    /// Return the combined stdout of `dxon <args>` as a `String`.
    fn stdout(&self, args: &[&str]) -> String {
        String::from_utf8_lossy(&self.run(args).stdout).into_owned()
    }

    /// Return the combined stderr of `dxon <args>` as a `String`.
    #[allow(dead_code)]
    fn stderr(&self, args: &[&str]) -> String {
        String::from_utf8_lossy(&self.run(args).stderr).into_owned()
    }

    /// Create a minimal fake container entry (directory + `meta.json`) so
    /// that read-only commands (`list`, `info`) can be tested without
    /// actually bootstrapping a real rootfs.
    fn create_fake_container(&self, name: &str, distro: &str) {
        let container_dir = self.containers_dir.join(name);
        let rootfs_dir = container_dir.join("rootfs");
        fs::create_dir_all(&rootfs_dir).expect("failed to create fake rootfs");

        let meta_json = format!(
            r#"{{
  "name": "{name}",
  "distro": "{distro}",
  "created_at": "2024-06-01T12:00:00Z",
  "template": null,
  "packages": ["git", "curl"],
  "repo": null,
  "rootfs_path": "{rootfs}",
  "config": {{
    "env": {{}},
    "extra_args": []
  }}
}}"#,
            rootfs = rootfs_dir.display()
        );

        let meta_path = container_dir.join("meta.json");
        fs::write(&meta_path, meta_json).expect("failed to write meta.json");
    }
}

// ---------------------------------------------------------------------------
// Basic CLI flag tests
// ---------------------------------------------------------------------------

#[test]
fn help_exits_zero() {
    let env = TestEnv::new();
    let out = env.run(&["--help"]);
    assert!(
        out.status.success(),
        "`dxon --help` should exit 0, got: {}",
        out.status
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("dxon") || stdout.contains("dXon"),
        "expected program name in --help output, got:\n{stdout}"
    );
}

#[test]
fn version_flag_shows_version() {
    let env = TestEnv::new();
    let out = env.run(&["--version"]);
    // clap exits 0 for --version
    assert!(
        out.status.success(),
        "`dxon --version` should exit 0, got: {}",
        out.status
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        combined.contains("0.") || combined.contains("dxon") || combined.contains("dXon"),
        "expected version string in output, got:\n{combined}"
    );
}

#[test]
fn no_subcommand_shows_help_and_fails() {
    let env = TestEnv::new();
    let out = env.run(&[]);
    // clap exits non-zero when no subcommand is given
    assert!(
        !out.status.success(),
        "`dxon` with no args should exit non-zero, got: {}",
        out.status
    );
}

// ---------------------------------------------------------------------------
// `dxon list` tests
// ---------------------------------------------------------------------------

#[test]
fn list_empty_store_exits_zero() {
    let env = TestEnv::new();
    let out = env.run(&["list"]);
    assert!(
        out.status.success(),
        "`dxon list` on empty store should exit 0, got: {}\nstderr: {}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn list_empty_store_says_no_containers() {
    let env = TestEnv::new();
    let stdout = env.stdout(&["list"]);
    assert!(
        stdout.contains("No containers"),
        "expected 'No containers' message in output, got:\n{stdout}"
    );
}

#[test]
fn list_shows_fake_container() {
    let env = TestEnv::new();
    env.create_fake_container("mybox", "arch");

    let stdout = env.stdout(&["list"]);
    assert!(
        stdout.contains("mybox"),
        "expected container name 'mybox' in list output, got:\n{stdout}"
    );
    assert!(
        stdout.contains("arch"),
        "expected distro 'arch' in list output, got:\n{stdout}"
    );
}

#[test]
fn list_shows_multiple_fake_containers() {
    let env = TestEnv::new();
    env.create_fake_container("box-a", "arch");
    env.create_fake_container("box-b", "debian");

    let stdout = env.stdout(&["list"]);
    assert!(stdout.contains("box-a"), "expected 'box-a' in list output");
    assert!(stdout.contains("box-b"), "expected 'box-b' in list output");
    assert!(stdout.contains("arch"), "expected 'arch' in list output");
    assert!(
        stdout.contains("debian"),
        "expected 'debian' in list output"
    );
}

// ---------------------------------------------------------------------------
// `dxon info` tests
// ---------------------------------------------------------------------------

#[test]
fn info_nonexistent_container_fails() {
    let env = TestEnv::new();
    let out = env.run(&["info", "ghost-container"]);
    assert!(
        !out.status.success(),
        "`dxon info <missing>` should exit non-zero"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("not found") || stderr.contains("ghost-container"),
        "expected 'not found' or the container name in error output, got:\n{stderr}"
    );
}

#[test]
fn info_fake_container_shows_details() {
    let env = TestEnv::new();
    env.create_fake_container("devbox", "debian");

    let out = env.run(&["info", "devbox"]);
    assert!(
        out.status.success(),
        "`dxon info devbox` should exit 0, got: {}\nstderr: {}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("devbox"),
        "expected container name 'devbox' in info output"
    );
    assert!(
        stdout.contains("debian"),
        "expected distro 'debian' in info output"
    );
}

#[test]
fn info_fake_container_shows_packages() {
    let env = TestEnv::new();
    env.create_fake_container("pkgbox", "arch");

    let stdout = env.stdout(&["info", "pkgbox"]);
    assert!(
        stdout.contains("git") || stdout.contains("curl"),
        "expected packages 'git' and/or 'curl' in info output, got:\n{stdout}"
    );
}

// ---------------------------------------------------------------------------
// `dxon delete` tests
// ---------------------------------------------------------------------------

#[test]
fn delete_nonexistent_container_fails() {
    let env = TestEnv::new();
    let out = env.run(&["delete", "ghost-container"]);
    // delete.rs calls std::process::exit(1) for missing containers
    assert!(
        !out.status.success(),
        "`dxon delete <missing>` should exit non-zero"
    );
}

#[test]
fn delete_force_removes_fake_container() {
    let env = TestEnv::new();
    env.create_fake_container("disposable", "arch");

    // Confirm the container directory exists before deletion.
    let container_dir = env.containers_dir.join("disposable");
    assert!(
        container_dir.exists(),
        "test setup: container dir should exist"
    );

    let out = env.run(&["delete", "--force", "disposable"]);
    assert!(
        out.status.success(),
        "`dxon delete --force disposable` failed: {}\nstderr: {}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );

    assert!(
        !container_dir.exists(),
        "container directory should be removed after delete"
    );
}

#[test]
fn delete_force_outputs_confirmation() {
    let env = TestEnv::new();
    env.create_fake_container("to-delete", "debian");

    let stdout = env.stdout(&["delete", "--force", "to-delete"]);
    assert!(
        stdout.contains("deleted") || stdout.contains("to-delete"),
        "expected deletion confirmation in output, got:\n{stdout}"
    );
}

// ---------------------------------------------------------------------------
// `dxon config` tests
// ---------------------------------------------------------------------------

#[test]
fn config_show_exits_zero() {
    let env = TestEnv::new();
    let out = env.run(&["config", "show"]);
    assert!(
        out.status.success(),
        "`dxon config show` should exit 0, got: {}\nstderr: {}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn config_show_displays_config_path() {
    let env = TestEnv::new();
    let stdout = env.stdout(&["config", "show"]);
    // Should mention the config file path
    assert!(
        stdout.contains("config.toml") || stdout.contains("dxon"),
        "expected config path or 'dxon' in config show output, got:\n{stdout}"
    );
}

#[test]
fn config_set_known_key_succeeds() {
    let env = TestEnv::new();

    let out = env.run(&["config", "set", "default_distro", "arch"]);
    assert!(
        out.status.success(),
        "`dxon config set default_distro arch` should exit 0, got: {}\nstderr: {}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn config_set_persists_and_shows_in_config_show() {
    let env = TestEnv::new();

    // Set the value.
    let set_out = env.run(&["config", "set", "default_distro", "debian"]);
    assert!(
        set_out.status.success(),
        "config set should succeed: {}",
        String::from_utf8_lossy(&set_out.stderr)
    );

    // Verify the value appears in `config show`.
    let stdout = env.stdout(&["config", "show"]);
    assert!(
        stdout.contains("debian"),
        "expected 'debian' in config show output after set, got:\n{stdout}"
    );
}

#[test]
fn config_set_multiple_keys_all_visible() {
    let env = TestEnv::new();

    env.run(&["config", "set", "default_distro", "alpine"]);
    env.run(&["config", "set", "default_shell", "zsh"]);

    let stdout = env.stdout(&["config", "show"]);
    assert!(
        stdout.contains("alpine"),
        "expected 'alpine' in config show output"
    );
    assert!(
        stdout.contains("zsh"),
        "expected 'zsh' in config show output"
    );
}

#[test]
fn config_set_unknown_key_fails() {
    let env = TestEnv::new();
    let out = env.run(&["config", "set", "bogus_key_that_does_not_exist", "value"]);
    assert!(
        !out.status.success(),
        "`dxon config set <unknown_key>` should exit non-zero"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("unknown") || stderr.contains("bogus_key") || stderr.contains("error"),
        "expected error mentioning the unknown key, got:\n{stderr}"
    );
}

#[test]
fn config_set_default_editor_succeeds() {
    let env = TestEnv::new();
    let out = env.run(&["config", "set", "default_editor", "code"]);
    assert!(
        out.status.success(),
        "`dxon config set default_editor code` should exit 0"
    );
}

#[test]
fn config_set_registry_url_succeeds() {
    let env = TestEnv::new();
    let out = env.run(&[
        "config",
        "set",
        "registry_url",
        "https://example.com/registry.json",
    ]);
    assert!(
        out.status.success(),
        "`dxon config set registry_url <url>` should exit 0, got: {}\nstderr: {}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );

    // Verify the value was persisted to the config TOML file directly,
    // since `config show` does not currently display registry_url.
    let config_path = env
        .home
        .path()
        .join(".config")
        .join("dxon")
        .join("config.toml");
    let config_contents =
        fs::read_to_string(&config_path).expect("config.toml should exist after config set");
    assert!(
        config_contents.contains("example.com"),
        "expected registry_url persisted in config.toml, got:\n{config_contents}"
    );
}

// ---------------------------------------------------------------------------
// `dxon template` tests (require network — ignored by default)
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires network access to the remote registry"]
fn template_list_exits_zero() {
    let env = TestEnv::new();
    let out = env.run(&["template", "list"]);
    assert!(
        out.status.success(),
        "`dxon template list` should exit 0, got: {}\nstderr: {}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
#[ignore = "requires network access to the remote registry"]
fn template_list_shows_templates() {
    let env = TestEnv::new();
    let stdout = env.stdout(&["template", "list"]);
    // The registry ships with at least rust and python templates.
    assert!(
        stdout.contains("rust") || stdout.contains("python") || stdout.contains("go"),
        "expected at least one well-known template in output, got:\n{stdout}"
    );
}

#[test]
#[ignore = "requires network access to the remote registry"]
fn template_search_rust_finds_result() {
    let env = TestEnv::new();
    let out = env.run(&["template", "search", "rust"]);
    assert!(
        out.status.success(),
        "`dxon template search rust` should exit 0, got: {}",
        out.status
    );
}

#[test]
#[ignore = "requires network access to the remote registry"]
fn template_refresh_exits_zero() {
    let env = TestEnv::new();
    let out = env.run(&["template", "refresh"]);
    assert!(
        out.status.success(),
        "`dxon template refresh` should exit 0, got: {}",
        out.status
    );
}

// ---------------------------------------------------------------------------
// `dxon create` tests (require system bootstrap tools — ignored in CI)
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires pacstrap / debootstrap / systemd-nspawn and root privileges"]
fn create_arch_container_full_lifecycle() {
    let env = TestEnv::new();

    // Create
    let create_out = env.run(&["create", "testbox", "--distro", "arch", "--trust"]);
    assert!(
        create_out.status.success(),
        "create failed:\n{}",
        String::from_utf8_lossy(&create_out.stderr)
    );

    // Info
    let info_stdout = env.stdout(&["info", "testbox"]);
    assert!(info_stdout.contains("testbox"));
    assert!(info_stdout.contains("arch"));

    // List
    let list_stdout = env.stdout(&["list"]);
    assert!(list_stdout.contains("testbox"));

    // Delete
    let delete_out = env.run(&["delete", "--force", "testbox"]);
    assert!(
        delete_out.status.success(),
        "delete failed:\n{}",
        String::from_utf8_lossy(&delete_out.stderr)
    );

    // Confirm gone
    let list_after = env.stdout(&["list"]);
    assert!(
        !list_after.contains("testbox"),
        "container should be gone after delete"
    );
}

// ---------------------------------------------------------------------------
// `--dir` flag / `DXON_DIR` override tests
// ---------------------------------------------------------------------------

#[test]
fn dir_flag_uses_specified_directory() {
    let env = TestEnv::new();
    // Create a second isolated containers directory.
    let alt_dir = env.home.path().join("alt-containers");
    fs::create_dir_all(&alt_dir).unwrap();

    // Manually populate it with a fake container.
    let container_dir = alt_dir.join("altbox");
    let rootfs = container_dir.join("rootfs");
    fs::create_dir_all(&rootfs).unwrap();
    let meta_json = format!(
        r#"{{
  "name": "altbox",
  "distro": "alpine",
  "created_at": "2024-06-01T12:00:00Z",
  "template": null,
  "packages": [],
  "repo": null,
  "rootfs_path": "{}",
  "config": {{"env": {{}}, "extra_args": []}}
}}"#,
        rootfs.display()
    );
    fs::write(container_dir.join("meta.json"), meta_json).unwrap();

    // Run `dxon list --dir <alt_dir>` — should find "altbox".
    let out = Command::new(dxon_bin())
        .args(["list", "--dir", alt_dir.to_str().unwrap()])
        .env("HOME", env.home.path())
        .env("NO_COLOR", "1")
        .env_remove("SUDO_USER")
        .output()
        .expect("failed to run dxon");

    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("altbox"),
        "expected 'altbox' when --dir points to alt directory, got:\n{stdout}"
    );
}
