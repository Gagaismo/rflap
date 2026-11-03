//! The `path` of `rflap_open_jff` and `rflap_save_jff`: `~`, `%VAR%` and the desktop that
//! OneDrive moves to `~/OneDrive/Desktop`.

use std::path::{Path, PathBuf};

/// The user's home folder: `USERPROFILE` on Windows, `HOME` elsewhere (the other as a
/// fallback).
fn home() -> Option<PathBuf> {
    let (first, second) = if cfg!(windows) {
        ("USERPROFILE", "HOME")
    } else {
        ("HOME", "USERPROFILE")
    };
    [first, second]
        .into_iter()
        .filter_map(std::env::var_os)
        .find(|value| !value.is_empty())
        .map(PathBuf::from)
}

/// Replaces `%NAME%` with the value of the environment variable, when it is set.
fn expand_variables(path: &str) -> String {
    let mut out = String::new();
    let mut rest = path;
    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find('%') {
            Some(end) if end > 0 => match std::env::var(&after[..end]) {
                Ok(value) => {
                    out.push_str(&value);
                    rest = &after[end + 1..];
                }
                Err(_) => {
                    out.push('%');
                    rest = after;
                }
            },
            _ => {
                out.push('%');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// A leading `~`, `~/` or `~\` as the home folder, with the separators of the platform.
fn expand_home(path: &str) -> PathBuf {
    let tail = match path.strip_prefix('~') {
        Some(tail) if tail.is_empty() || tail.starts_with(['/', '\\']) => tail,
        _ => return PathBuf::from(path),
    };
    match home() {
        Some(mut expanded) => {
            expanded.extend(Path::new(tail.trim_start_matches(['/', '\\'])).components());
            expanded
        }
        None => PathBuf::from(path),
    }
}

/// The path the user means. Returns it with a remark when it differs from the one given in more
/// than the expansion (the desktop of OneDrive).
pub fn resolve(path: &str) -> (PathBuf, Option<String>) {
    let mut text = path.to_string();
    if cfg!(windows) {
        text = expand_variables(&text);
    }
    let expanded = expand_home(&text);
    let Some(home) = home() else {
        return (expanded, None);
    };
    // `<home>/Desktop/...` when that folder does not exist but OneDrive's does
    let Ok(under) = expanded.strip_prefix(home.join("Desktop")) else {
        return (expanded, None);
    };
    if home.join("Desktop").exists() {
        return (expanded, None);
    }
    for folder in ["Desktop", "Escritorio"] {
        let desktop = home.join("OneDrive").join(folder);
        if desktop.is_dir() {
            let moved = desktop.join(under);
            let remark = format!(
                "El escritorio no está en `{}` sino en OneDrive; se usó `{}`.",
                home.join("Desktop").display(),
                moved.display()
            );
            return (moved, Some(remark));
        }
    }
    (expanded, None)
}
