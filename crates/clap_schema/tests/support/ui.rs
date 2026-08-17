//! Compiler-UI test support.

use std::{
    env,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::{self, Output},
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

use snapbox::cmd::Command;

static CARGO_LOCK: Mutex<()> = Mutex::new(());
static NEXT_PROJECT: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug)]
struct UiProject {
    root: PathBuf,
    diagnostic_path: String,
}

impl UiProject {
    fn new(fixture: &str) -> Self {
        let id = NEXT_PROJECT.fetch_add(1, Ordering::Relaxed);
        let root = env::temp_dir().join(format!("clap_schema-ui-{}-{id}-{fixture}", process::id()));
        if root.exists() {
            fs::remove_dir_all(&root).unwrap_or_else(|error| {
                panic!("failed to clear temporary UI project `{}`: {error}", root.display())
            });
        }
        fs::create_dir_all(root.join("src")).unwrap_or_else(|error| {
            panic!("failed to create temporary UI project `{}`: {error}", root.display())
        });

        let facade = repository_root().to_string_lossy().replace('\\', "/");
        let manifest = format!(
            "[package]\nname = \"clap_schema-ui-{fixture}\"\nversion = \"0.0.0\"\nedition = \"2024\"\npublish = false\n\n[dependencies]\nclap = {{ version = \"4.6.6\", features = [\"derive\"] }}\nclap_schema = {{ path = \"{facade}\" }}\n"
        );
        fs::write(root.join("Cargo.toml"), manifest).unwrap_or_else(|error| {
            panic!("failed to write temporary UI manifest `{}`: {error}", root.display())
        });

        let source = repository_root().join("tests/fixtures/ui/fail").join(format!("{fixture}.rs"));
        fs::copy(&source, root.join("src/main.rs")).unwrap_or_else(|error| {
            panic!("failed to copy UI fixture `{}`: {error}", source.display())
        });

        Self { root, diagnostic_path: format!("fail/{fixture}.rs") }
    }

    fn command(&self) -> Command {
        let cargo = env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"));
        Command::new(cargo)
            .current_dir(&self.root)
            .env("CARGO_TARGET_DIR", repository_root().join("target/ui-tests"))
            .args(["check", "--quiet", "--offline", "--color", "never"])
    }
}

impl Drop for UiProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// Compiles one downstream fixture and returns normalized compiler output.
#[track_caller]
pub(crate) fn ui_output(fixture: &str) -> Output {
    let _guard = CARGO_LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let project = UiProject::new(fixture);
    let output = project
        .command()
        .output()
        .unwrap_or_else(|error| panic!("failed to compile UI fixture `{fixture}`: {error}"));
    normalize_output(output, &project.diagnostic_path)
}

fn normalize_output(mut output: Output, diagnostic_path: &str) -> Output {
    let stderr = String::from_utf8_lossy(&output.stderr).replace('\\', "/");
    let mut lines = stderr
        .lines()
        .filter(|line| !line.starts_with("error: could not compile `clap_schema-ui-"))
        .filter(|line| !line.starts_with("error: process didn't exit successfully:"))
        .filter(|line| {
            let Some((_, marker)) = line.split_once('|') else {
                return true;
            };
            let marker = marker.trim();
            marker.is_empty() || !marker.chars().all(|character| character == '^')
        })
        .collect::<Vec<_>>();
    while lines.last().is_some_and(|line| line.trim().is_empty()) {
        lines.pop();
    }

    let mut normalized = lines.join("\n").replace("src/main.rs", diagnostic_path);
    if !normalized.is_empty() {
        normalized.push('\n');
    }
    output.stderr = normalized.into_bytes();
    output
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}
