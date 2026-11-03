//! Concatenates the ES modules of `web/src` into one script.
//!
//! The sources are plain ES modules (so `node --test` imports them directly). The browser build
//! has no module loader, so this walks the imports from the entry file, emits every module once
//! in dependency order and removes `import` lines and the `export` keyword. That only works for
//! a restricted syntax, which is checked and reported with the file and line:
//!
//! * `import { a, b } from './x.js';` (the names may span several lines) or `import './x.js';`, no `as`, no `*`;
//! * `export function|async function|const|let|var|class …` or `export { a, b };`;
//! * top-level names unique across modules (a duplicate is a `SyntaxError` the test suite catches).

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

/// The bundled script for `entry` (a path relative to `src`).
pub fn bundle(src: &Path, entry: &str) -> Result<String, String> {
    let mut done = BTreeSet::new();
    let mut out = String::new();
    visit(src, entry, &mut done, &mut Vec::new(), &mut out)?;
    if out.to_ascii_lowercase().contains("</script") {
        return Err("the bundle contains `</script`, which would end the inline script".into());
    }
    Ok(out)
}

fn visit(
    src: &Path,
    name: &str,
    done: &mut BTreeSet<String>,
    stack: &mut Vec<String>,
    out: &mut String,
) -> Result<(), String> {
    if done.contains(name) {
        return Ok(());
    }
    if stack.iter().any(|s| s == name) {
        return Err(format!("import cycle: {} -> {name}", stack.join(" -> ")));
    }
    stack.push(name.to_string());
    let text = fs::read_to_string(src.join(name)).map_err(|e| format!("{name}: {e}"))?;
    let mut body = String::new();
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let at = format!("{name}:{}", i + 1);
        let first = lines[i];
        i += 1;
        if first.starts_with("import ") || first.starts_with("import{") {
            // a list of names may continue over several lines up to the closing brace
            let mut joined = first.to_string();
            while joined.contains('{') && !joined.contains('}') && i < lines.len() {
                joined.push(' ');
                joined.push_str(lines[i].trim());
                i += 1;
            }
            let line = joined.as_str();
            let dep = import_target(line).ok_or_else(|| {
                format!("{at}: unsupported import (one `import {{ a }} from './x.js';` per line, no `as` or `*`): {line}")
            })?;
            visit(src, &join(name, &dep), done, stack, out)?;
        } else if first.starts_with("export") {
            let stripped =
                strip_export(first).ok_or_else(|| format!("{at}: unsupported export: {first}"))?;
            if !stripped.is_empty() {
                body.push_str(&stripped);
                body.push('\n');
            }
        } else {
            body.push_str(first);
            body.push('\n');
        }
    }
    stack.pop();
    done.insert(name.to_string());
    out.push_str(&format!("// ---- {name}\n{body}\n"));
    Ok(())
}

/// The path inside the quotes of a supported import line.
fn import_target(line: &str) -> Option<String> {
    let rest = line.strip_prefix("import")?.trim_start();
    let from = if let Some(rest) = rest.strip_prefix('{') {
        let (names, tail) = rest.split_once('}')?;
        let plain = names
            .split(',')
            .map(str::trim)
            .filter(|n| !n.is_empty())
            .all(|n| {
                n.chars()
                    .all(|c| c.is_alphanumeric() || c == '_' || c == '$')
            });
        if !plain {
            return None;
        }
        tail.trim_start().strip_prefix("from")?.trim_start()
    } else {
        rest
    };
    let quote = from.chars().next().filter(|c| *c == '\'' || *c == '"')?;
    let (path, tail) = from[1..].split_once(quote)?;
    let tail = tail.trim();
    if !(tail.is_empty() || tail == ";" || tail.starts_with("//") || tail.starts_with(";")) {
        return None;
    }
    (path.starts_with("./") || path.starts_with("../")).then(|| path.to_string())
}

/// Resolves `./x.js` against the importing file's directory (normalizing `..`).
fn join(from: &str, rel: &str) -> String {
    let mut parts: Vec<&str> = from.split('/').collect();
    parts.pop();
    for piece in rel.split('/') {
        match piece {
            "." | "" => {}
            ".." => {
                parts.pop();
            }
            p => parts.push(p),
        }
    }
    parts.join("/")
}

/// The line without `export`, or an empty string for `export { a, b };`; `None` if unsupported.
fn strip_export(line: &str) -> Option<String> {
    let rest = line.strip_prefix("export ")?.trim_start();
    if let Some(list) = rest.strip_prefix('{') {
        let (names, tail) = list.split_once('}')?;
        let plain = names
            .split(',')
            .map(str::trim)
            .filter(|n| !n.is_empty())
            .all(|n| {
                n.chars()
                    .all(|c| c.is_alphanumeric() || c == '_' || c == '$')
            });
        return (plain && !tail.contains("from")).then(String::new);
    }
    [
        "function ",
        "function* ",
        "async function ",
        "const ",
        "let ",
        "var ",
        "class ",
    ]
    .iter()
    .any(|k| rest.starts_with(k))
    .then(|| rest.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str, files: &[(&str, &str)]) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("rflap-xtask-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        for (file, text) in files {
            if let Some(parent) = Path::new(file).parent() {
                fs::create_dir_all(dir.join(parent)).unwrap();
            }
            fs::write(dir.join(file), text).unwrap();
        }
        dir
    }

    #[test]
    fn orders_dependencies_first_and_strips_syntax() {
        let dir = tmp(
            "order",
            &[
                (
                    "main.js",
                    "import { a } from './lib/a.js';\nimport './b.js';\nexport const m = a;\n",
                ),
                (
                    "lib/a.js",
                    "import { b } from '../b.js';\nexport function a() { return b; }\n",
                ),
                ("b.js", "export const b = 1;\nexport { b };\n"),
            ],
        );
        let out = bundle(&dir, "main.js").unwrap();
        let (b, a, m) = (
            out.find("// ---- b.js").unwrap(),
            out.find("// ---- lib/a.js").unwrap(),
            out.find("// ---- main.js").unwrap(),
        );
        assert!(b < a && a < m);
        assert!(!out.contains("import ") && !out.contains("export "));
        assert!(out.contains("const m = a;") && out.contains("function a()"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn imports_may_span_lines() {
        let dir = tmp(
            "multiline",
            &[
                (
                    "main.js",
                    "import {
  a,
  b,
} from './x.js';
console.log(a, b);
",
                ),
                (
                    "x.js",
                    "export const a = 1;
export const b = 2;
",
                ),
            ],
        );
        let out = bundle(&dir, "main.js").unwrap();
        assert!(out.contains("console.log(a, b);") && !out.contains("import"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn modules_are_emitted_once() {
        let dir = tmp(
            "once",
            &[
                (
                    "main.js",
                    "import { x } from './x.js';\nimport { y } from './y.js';\n",
                ),
                ("x.js", "import { z } from './z.js';\nexport const x = z;\n"),
                ("y.js", "import { z } from './z.js';\nexport const y = z;\n"),
                ("z.js", "export const z = 1;\n"),
            ],
        );
        assert_eq!(
            bundle(&dir, "main.js")
                .unwrap()
                .matches("// ---- z.js")
                .count(),
            1
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn rejects_what_it_cannot_bundle() {
        for (name, source) in [
            ("alias", "import { a as b } from './x.js';\n"),
            ("star", "import * as x from './x.js';\n"),
            ("bare", "import x from 'lodash';\n"),
            ("default", "export default 1;\n"),
            ("reexport", "export * from './x.js';\n"),
            ("aliasexport", "export { a as b };\n"),
            ("script", "const s = '</script>';\n"),
        ] {
            let dir = tmp(
                name,
                &[("main.js", source), ("x.js", "export const a = 1;\n")],
            );
            assert!(bundle(&dir, "main.js").is_err(), "{name}");
            let _ = fs::remove_dir_all(dir);
        }
    }

    #[test]
    fn detects_cycles() {
        let dir = tmp(
            "cycle",
            &[
                ("main.js", "import { a } from './a.js';\n"),
                ("a.js", "import { b } from './b.js';\nexport const a = 1;\n"),
                ("b.js", "import { a } from './a.js';\nexport const b = 1;\n"),
            ],
        );
        assert!(bundle(&dir, "main.js").unwrap_err().contains("cycle"));
        let _ = fs::remove_dir_all(dir);
    }
}
