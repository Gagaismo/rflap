# Rflap in Claude Code

Claude Code runs in a terminal, so it cannot draw the interactive widget. It gets, for every
visual result, a Markdown summary with the transition table and the verification report, and
(optionally) a link that opens the same automaton in your browser.

## 1. Get the server

Download `rflap-mcp` for your system from https://github.com/Gagaismo/rflap/releases/latest and unzip it, or build it from source:

```powershell
cargo build --release -p rflap-mcp
# -> target\release\rflap-mcp.exe
```

## 2. Register it (stdio, recommended)

```powershell
claude mcp add rflap -- C:\path\to\rflap-mcp.exe --viewer-port 8787
```

Everything after `--` is the server command. `--viewer-port 8787` makes results end with a link
such as `http://127.0.0.1:8787/#doc=...` that opens the automaton in the browser widget. If the
port is busy (a second Claude Code session) the server keeps working without the link; use
`--viewer-port 0` to take any free port.

Useful variants:

```powershell
claude mcp add --scope user rflap -- C:\path\to\rflap-mcp.exe --viewer-port 8787   # all projects
claude mcp add --scope project rflap -- C:\path\to\rflap-mcp.exe                   # shared via .mcp.json
claude mcp list
claude mcp get rflap
claude mcp remove rflap
```

## 3. Register it over HTTP (alternative)

Start the server once in a terminal (it also serves the viewer at `/`):

```powershell
C:\path\to\rflap-mcp.exe --http 8787
```

and add it:

```powershell
claude mcp add --transport http rflap http://127.0.0.1:8787/mcp
```

## 4. Try it

Inside Claude Code, run `/mcp` to check that `rflap` is connected, then ask:

> Design an automaton for the strings over {a,b} that end in `ab`, and show me the table.

Claude should reason about the language, call `rflap_design_automaton` with a formal spec plus
accept/reject examples, read the verification, and explain the states. The MCP prompt
`ssl_exercise` (shown as `/mcp__rflap__ssl_exercise`) starts that workflow from an exercise
statement.

For a workflow that does not depend on the MCP server being loaded, install the agent skill
`skills/rflap/SKILL.md` (copy the `skills/rflap` folder to `~/.claude/skills/`).

## Notes

* Tool names are `rflap_*`; the reference is in [../mcp-tools.md](../mcp-tools.md).
* `rflap_open_jff` / `rflap_save_jff` accept a `path` in stdio mode, so Claude can read and write
  `.jff` files for JFLAP; pass `--no-files` to disable that.
* Verified against the Claude Code documentation (`claude mcp add`, `--transport http`, `--`
  separator, scopes). Not verified: behaviour of old Claude Code versions.
