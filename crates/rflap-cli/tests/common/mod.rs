//! Helpers shared by the integration tests: run the binary, temp directories.
#![allow(dead_code)]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct Out {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

/// The compact DFA of `(ab|…)` used by most tests: q0 -a-> q1 -b-> q2 -a-> q1, accepting q2.
pub const DFA: &str = r#"{"type":"fa","initial":"q0","finals":["q2"],"transitions":[["q0","a","q1"],["q1","b","q2"],["q2","a","q1"]]}"#;

/// The NFA of `(a+b)*abb`.
pub const NFA: &str = r#"{"type":"fa","initial":"q0","finals":["q3"],"transitions":[["q0","a","q0"],["q0","b","q0"],["q0","a","q1"],["q1","b","q2"],["q2","b","q3"]]}"#;

pub fn fixture(name: &str) -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
        .to_string_lossy()
        .into_owned()
}

pub fn rflap(args: &[&str]) -> Out {
    rflap_stdin(args, "")
}

pub fn rflap_stdin(args: &[&str], stdin: &str) -> Out {
    let mut child = Command::new(env!("CARGO_BIN_EXE_rflap"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the binary starts");
    child
        .stdin
        .take()
        .expect("stdin is piped")
        .write_all(stdin.as_bytes())
        .ok();
    let out = child.wait_with_output().expect("the binary finishes");
    Out {
        code: out.status.code().expect("exited normally"),
        stdout: String::from_utf8(out.stdout).expect("stdout is UTF-8"),
        stderr: String::from_utf8(out.stderr).expect("stderr is UTF-8"),
    }
}

/// A unique temp directory removed when dropped.
pub struct Temp(PathBuf);

static COUNTER: AtomicUsize = AtomicUsize::new(0);

impl Temp {
    pub fn new() -> Temp {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("rflap-cli-test-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        Temp(dir)
    }

    pub fn path(&self, name: &str) -> String {
        self.0.join(name).to_string_lossy().into_owned()
    }

    /// Writes a file and returns its path.
    pub fn write(&self, name: &str, content: &str) -> String {
        std::fs::write(self.0.join(name), content).expect("write");
        self.path(name)
    }

    pub fn read(&self, name: &str) -> String {
        std::fs::read_to_string(self.0.join(name)).expect("read")
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}
