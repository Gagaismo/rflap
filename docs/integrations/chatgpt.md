# Rflap in ChatGPT

ChatGPT shows Rflap's interactive widget (an MCP Apps UI) next to the answer. ChatGPT connects to
MCP servers from OpenAI's side, so the server needs a public HTTPS URL (or OpenAI's Secure MCP
Tunnel).

## 1. Run the server and publish it

```powershell
cargo build --release -p rflap-mcp
C:\path\to\rflap-mcp.exe --http 8787
# in another terminal, any HTTPS forwarder, for example:
cloudflared tunnel --url http://127.0.0.1:8787     # prints https://<random>.trycloudflare.com
```

The MCP endpoint is `https://<random>.trycloudflare.com/mcp` (the `/mcp` path is required).

## 2. Add it in ChatGPT

Following OpenAI's "Connect and test your plugin" guide:

1. Go to <https://chatgpt.com/plugins>.
2. Select the plus button, then **Add custom MCP server**.
3. Enter a name and description (for example "Rflap: automata and grammars").
4. Under **Connection** enter the MCP server URL including `/mcp`.
5. Authentication: none (Rflap has none). Review the risk warning and continue.
6. **Create as a plugin**, then check that the tools (`rflap_*`) were discovered.
7. Start a new conversation, type `@` and pick the plugin, then ask: *"Design an automaton for
   the strings over {a,b} that end in ab."*

After rebuilding the server with different tools, open the connection in ChatGPT Plugins and
press **Refresh**. To debug raw requests, use MCP Inspector
(`npx @modelcontextprotocol/inspector@latest`) or the API Playground (*Tools, Add, MCP Server*).

## What ChatGPT receives

* Every visual tool has `_meta.ui.resourceUri = "ui://rflap/editor.html"` (the MCP Apps standard)
  plus the ChatGPT aliases `openai/outputTemplate`, `openai/widgetAccessible` and
  `openai/toolInvocation/invoking|invoked` (short status text).
* The resource `ui://rflap/editor.html` has MIME type `text/html;profile=mcp-app`, an empty CSP
  (the page is self-contained and makes no network requests) and `prefersBorder`; the legacy
  `openai/widgetCSP`, `openai/widgetDescription` and `openai/widgetPrefersBorder` aliases are
  included too.
* The tool result's `structuredContent` feeds the widget; `content` is the Markdown fallback.

## Notes

* Tool reference: [../mcp-tools.md](../mcp-tools.md). The tunnel URL is unauthenticated: close it
  when you finish, and keep file access off (`--allow-files` is off by default over HTTP).
* Verified in OpenAI's Apps/Plugins documentation: `_meta.ui.resourceUri` with the
  `openai/outputTemplate` alias, `text/html;profile=mcp-app`, `openai/toolInvocation/*` (at most
  64 characters), `openai/widgetAccessible`, `openai/widgetPrefersBorder`, the snake_case fields
  of `openai/widgetCSP`, and the *Add custom MCP server* flow above.
* **UNVERIFIED**: which ChatGPT plans or workspace settings allow custom MCP servers (OpenAI says
  account and workspace policies apply); how ChatGPT mobile renders the widget; that this exact
  widget renders (needs a manual test once `web/dist/rflap.html` is the final build).
