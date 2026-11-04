//! The `path` of `rflap_save_jff` and `rflap_open_jff`: `~`, `%VAR%` and the desktop in
//! OneDrive. The real server binary runs with a temporary home, so the real one is never
//! touched.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::{Value, json};

fn temp_home(name: &str) -> PathBuf {
    let home = std::env::temp_dir().join(format!("rflap-paths-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    home
}

/// One `tools/call` answered by a fresh server whose home folder is `home`.
fn call(home: &Path, name: &str, arguments: Value) -> Value {
    let mut child = Command::new(env!("CARGO_BIN_EXE_rflap-mcp"))
        .env("HOME", home)
        .env("USERPROFILE", home)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("the binary starts");
    let mut stdin = child.stdin.take().unwrap();
    for line in [
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize",
               "params": {"protocolVersion": "2025-06-18", "capabilities": {},
                          "clientInfo": {"name": "t", "version": "1"}}}),
        json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
        json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call",
               "params": {"name": name, "arguments": arguments}}),
    ] {
        writeln!(stdin, "{line}").unwrap();
    }
    drop(stdin);
    let mut out = String::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut out)
        .unwrap();
    child.wait().unwrap();
    out.lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .find(|v| v["id"] == 2)
        .map(|v| v["result"].clone())
        .unwrap_or_else(|| panic!("no answer: {out}"))
}

fn machine() -> Value {
    json!({"type": "turing", "initial": "q0", "finals": ["q1"],
           "transitions": [["q0", "(a,<,D)", "q1"], ["q1", "(c,>,I)", "q0"]]})
}

fn text(result: &Value) -> String {
    result["content"][0]["text"].as_str().unwrap().to_string()
}

fn save(home: &Path, path: &str) -> Value {
    call(
        home,
        "rflap_save_jff",
        json!({"document": machine(), "path": path}),
    )
}

#[test]
fn s5_a_leading_tilde_is_the_home_folder() {
    let home = temp_home("tilde");
    let r = save(&home, "~/maquina.jff");
    assert_eq!(r["isError"], false, "{r}");
    let saved = home.join("maquina.jff");
    assert!(saved.exists());
    assert_eq!(
        r["structuredContent"]["path"],
        json!(saved.display().to_string())
    );
    assert!(text(&r).contains(&saved.display().to_string()), "{r}");
    // and it reads back through open_jff with the same `~`
    let opened = call(&home, "rflap_open_jff", json!({"path": "~/maquina.jff"}));
    assert_eq!(opened["isError"], false, "{opened}");
    assert_eq!(opened["structuredContent"]["document"]["type"], "turing");
    let _ = std::fs::remove_dir_all(&home);
}

#[cfg(windows)]
#[test]
fn s5_tilde_backslash_and_percent_variables_work_on_windows() {
    let home = temp_home("windows");
    let r = save(&home, "~\\uno.jff");
    assert_eq!(r["isError"], false, "{r}");
    assert!(home.join("uno.jff").exists());
    let r = save(&home, "%USERPROFILE%\\dos.jff");
    assert_eq!(r["isError"], false, "{r}");
    assert!(home.join("dos.jff").exists());
    let opened = call(
        &home,
        "rflap_open_jff",
        json!({"path": "%USERPROFILE%\\dos.jff"}),
    );
    assert_eq!(opened["isError"], false, "{opened}");
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn s5_the_desktop_inside_onedrive_is_used_when_the_plain_one_is_missing() {
    for folder in ["Desktop", "Escritorio"] {
        let home = temp_home(folder);
        std::fs::create_dir_all(home.join("OneDrive").join(folder)).unwrap();
        let r = save(&home, "~/Desktop/ejercicio.jff");
        assert_eq!(r["isError"], false, "{folder}: {r}");
        let real = home.join("OneDrive").join(folder).join("ejercicio.jff");
        assert!(real.exists(), "{folder}");
        assert!(!home.join("Desktop").exists(), "{folder}");
        let shown = text(&r);
        assert!(shown.contains(&real.display().to_string()), "{shown}");
        assert!(shown.contains("OneDrive"), "{shown}");
        assert_eq!(
            r["structuredContent"]["path"],
            json!(real.display().to_string())
        );
        let opened = call(
            &home,
            "rflap_open_jff",
            json!({"path": "~/Desktop/ejercicio.jff"}),
        );
        assert_eq!(opened["isError"], false, "{folder}: {opened}");
        let _ = std::fs::remove_dir_all(&home);
    }
}

#[test]
fn s5_an_existing_plain_desktop_wins_over_onedrive() {
    let home = temp_home("both");
    std::fs::create_dir_all(home.join("Desktop")).unwrap();
    std::fs::create_dir_all(home.join("OneDrive").join("Desktop")).unwrap();
    let r = save(&home, "~/Desktop/x.jff");
    assert_eq!(r["isError"], false, "{r}");
    assert!(home.join("Desktop").join("x.jff").exists());
    assert!(!home.join("OneDrive").join("Desktop").join("x.jff").exists());
    assert!(!text(&r).contains("OneDrive"));
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn s5_without_any_desktop_the_error_names_the_expanded_path() {
    let home = temp_home("none");
    let r = save(&home, "~/Desktop/x.jff");
    assert_eq!(r["isError"], true, "{r}");
    let shown = text(&r);
    assert!(shown.contains("could not write"), "{shown}");
    assert!(!shown.contains('~'), "{shown}");
    assert!(shown.contains("Desktop"), "{shown}");
    let _ = std::fs::remove_dir_all(&home);
}
