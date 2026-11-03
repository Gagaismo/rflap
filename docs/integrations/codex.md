# Rflap in Codex

Codex (CLI and IDE extension) runs in a terminal-like environment: it receives Markdown
(summary, transition table, verification report) and an optional viewer link, not the widget.

## 1. Get the server

Download `rflap-mcp` for your system from https://github.com/Gagaismo/rflap/releases/latest and unzip it, or build it from source:

```powershell
cargo build --release -p rflap-mcp
```

## 2. Register it

One command:

```powershell
codex mcp add rflap -- C:\path\to\rflap-mcp.exe --viewer-port 8787
```

or edit `~/.codex/config.toml` (project-scoped: `.codex/config.toml`):

```toml
[mcp_servers.rflap]
command = 'C:\path\to\rflap-mcp.exe'
args = ["--viewer-port", "8787"]
startup_timeout_sec = 20
```

Single quotes keep the Windows backslashes literal. Over HTTP instead, start
`rflap-mcp --http 8787` and use:

```toml
[mcp_servers.rflap]
url = "http://127.0.0.1:8787/mcp"
```

## 3. Try it

Ask Codex:

> Use the rflap tools: design a DFA for the strings over {a,b} with an even number of a's,
> verify it with examples, and show me the transition table.

Codex should call `rflap_design_automaton` (regular languages) or the `an_bn` template
(non-regular ones) and report the verification. For a ready-made workflow, copy
`skills/rflap/SKILL.md` into your Codex skills folder.

## Notes

* Tool reference: [../mcp-tools.md](../mcp-tools.md).
* Verified against the Codex documentation: `codex mcp add <name> -- <command>`, the
  `[mcp_servers.<name>]` table with `command`, `args`, `startup_timeout_sec` and `url`.
  **UNVERIFIED**: whether the Codex app (as opposed to the CLI) renders MCP Apps widgets.
