# Rflap in Claude Desktop and Claude.ai

Both can render the interactive Rflap widget inside the conversation (MCP Apps). The MCP
documentation lists Claude and Claude Desktop among the hosts that support MCP Apps.

## Claude Desktop with a local server (stdio)

1. Download `rflap-mcp` from https://github.com/Gagaismo/rflap/releases/latest and unzip it
   (or build it: `cargo build --release -p rflap-mcp`).
2. In Claude Desktop open the Claude menu → *Settings…* → *Developer* → *Edit Config*, which opens
   `claude_desktop_config.json` (`%APPDATA%\Claude\claude_desktop_config.json` on Windows,
   `~/Library/Application Support/Claude/claude_desktop_config.json` on macOS).
3. Add the server (double the backslashes in JSON):

```json
{
  "mcpServers": {
    "rflap": {
      "command": "C:\\path\\to\\rflap-mcp.exe",
      "args": ["--viewer-port", "8787"]
    }
  }
}
```

   **Windows, installer from claude.ai (MSIX package):** this build does not read
   `%APPDATA%\Claude\claude_desktop_config.json`. Its config file is
   `%LOCALAPPDATA%\Claude-Data\claude_<id>\claude_desktop_config.json` (for example
   `claude_pzs8sxrjxfjjc`); *Edit Config* opens the right one. Add `mcpServers` there, next to the
   keys the app already keeps, and edit it only while Claude Desktop is closed (the app rewrites
   the file on exit).

4. Quit Claude Desktop completely and start it again. Ask: *"Design an automaton for the strings
   over {a,b} that end in ab"* and the widget should appear below the answer. Chats that cannot
   show it still get the Markdown table and the verification report.

## Claude.ai (web) and Claude Desktop with a remote server

Claude.ai connects to remote MCP servers from Anthropic's servers, so the server must be reachable
from the internet over HTTPS. Local servers configured in `claude_desktop_config.json` are a
separate mechanism and are not available in claude.ai.

1. Start Rflap over HTTP: `rflap-mcp --http 8787` (binds `127.0.0.1` only).
2. Publish it with a tunnel that gives you an HTTPS URL, for example
   `cloudflared tunnel --url http://127.0.0.1:8787` (prints `https://<random>.trycloudflare.com`).
3. In Claude: *Customize → Connectors → + Add → Add custom connector*, name it `Rflap`, and enter
   `https://<random>.trycloudflare.com/mcp` as the remote MCP server URL. Continue; Rflap has no
   authentication, so choose no authentication.
4. In a chat, enable the connector from the tools menu and ask for an automaton.

Plans: the support article says individual **Pro** and **Max** (and Ultra) plans add custom
connectors themselves, and on **Team** and **Enterprise** only Owners (or custom roles with the
permission) can add them. **UNVERIFIED**: whether the Free plan can add custom connectors, and the
exact current limits of each plan.

### Security of the tunnel

The URL is the only secret: anyone who has it can run Rflap operations (they are pure
computations, but they cost CPU). Over HTTP the file arguments of `rflap_open_jff` and
`rflap_save_jff` are disabled unless you pass `--allow-files`; do not pass it when tunnelling.
Close the tunnel when you are done. Browsers' `Origin` headers are checked: loopback origins are
accepted, others only with `--allow-origin https://example.com`.

## Notes

* Tool reference: [../mcp-tools.md](../mcp-tools.md).
* Turing machines and ALL in the notation of the course (`rflap_mt_catedra`, prompt `mt_catedra`):
  [../catedra.md](../catedra.md).
* Verified: the Claude support article for custom connectors (steps above), the MCP "connect to
  local servers" guide for `claude_desktop_config.json`, the MCP Apps client list.
  **UNVERIFIED** here: that this exact widget renders in each Claude host (needs a manual test
  once the web build `web/dist/rflap.html` is final).

## Logs

`rflap-mcp` writes one line to stderr per tool call (tool, `ok`/`error`, time, arguments and the
first line of the answer; `--quiet` turns it off). Claude Desktop keeps it in
`mcp-server-rflap.log`: `%LOCALAPPDATA%\Claude\Logs\` for the MSIX build on Windows,
`%APPDATA%\Claude\logs\` for the classic one, `~/Library/Logs/Claude/` on macOS.
