# Rflap web editor

The graphical interface of Rflap: a vanilla-JS app (no dependencies, no bundler, no network) that runs the
`rflap-api` engine as WebAssembly. **One self-contained file**, `web/dist/rflap.html`, is used

1. standalone in a browser (`rflap gui`),
2. as the MCP Apps view rendered inside Claude and other MCP Apps hosts (`ui://` resource),
3. as the widget of ChatGPT (`window.openai` bridge).

```text
cargo run -p xtask -- build-web          # builds rflap-wasm, writes web/dist/rflap.html (commit it)
node --test web/test/*.test.js           # JS tests (the CI runs build-web first)
```

> `node --test web/test/` (a directory) does not work on Node 22+, which treats the argument as a file;
> use the glob above.

`web/dist/rflap.html` is what `rflap gui` and `rflap-mcp` embed with `include_str!`. It has inline CSS, the
bundled JS and the wasm as base64; the build fails if it would reference any external URL.

## Layout

| file (`web/src/`) | what |
|---|---|
| `index.html`, `style.css` | template (the xtask fills three markers) and styles (light/dark, responsive) |
| `main.js` | entry: host, engine (wasm or remote), app shell |
| `wasm.js` | wasm loader: `decodeBase64`, `loadWasm(bytes) -> call(text) -> text` over the C ABI of `rflap-wasm` |
| `api.js` | `createApi(getTransport)` -> `api(request) -> result` (throws `ApiError` with the engine's `code`); `localTransport`, `remoteTransport` |
| `doc.js` | helpers over the FULL document JSON (add/remove state/transition, blocks, labels, compact form, `History`) |
| `share.js` | `#doc=<base64url JSON>` links |
| `host.js` | host bridge: `createHost(window, handlers)` for standalone, MCP Apps and ChatGPT |
| `i18n.js` | English and Spanish; `t(key, params)`, `registerStrings`, `pickLocale` |
| `registry.js` | **panel and action registries** (below) |
| `tab.js` | `Tab`: a document with undo history, flags, overlays; it is the `ctx` of panels and actions |
| `app.js` | shell: menu bar, tabs, workspaces, files, host integration |
| `editor.js` | the canvas (engine SVG made interactive through `data-state-id`, `data-transition-index`, ...) |
| `labels.js`, `dialogs.js`, `dom.js` | label forms per type, modals/menus/toasts, DOM helpers |
| `sim.js`, `structure.js`, `analysis.js`, `steps.js`, `assistant.js`, `info.js` | built-in panels |
| `actions.js` | File / Edit / Input / Test / Convert / Layout / View / Help menu actions |

The JS files are plain ES modules (so node can import them in tests). `xtask` concatenates them in dependency
order and strips `import`/`export`, which needs a restricted syntax: `import { a, b } from './x.js';` (the names
may span lines; no `as`, no `*`, no default import), `export function|const|let|class …` or `export { a, b };`, and
top-level names unique across modules (`web/test/build.test.js` catches a clash).

## The engine

`api(request)` is async so that the same code runs on local wasm and, when needed, on the MCP server:

* **Local wasm** (default): `rflap_alloc / rflap_call / rflap_free / rflap_free_result`, no wasm-bindgen.
* **Remote fallback**: the default MCP Apps CSP is `script-src 'self' 'unsafe-inline'` with **no
  `'wasm-unsafe-eval'`**, so a strict host may refuse to compile the module. When that happens the view calls the
  server through `tools/call` with the tool `rflap_call` (override: `window.RFLAP_REMOTE_TOOL`) and the argument
  `{request: "<rflap-api request JSON>"}`; the tool must answer the `rflap_api::call` JSON as `structuredContent`
  (or as JSON text content). Hosts that allow wasm never take this path.

Documents are handled in the **full form** (what `normalize` returns). Anything that arrives in another form
(`.jff`, compact JSON from an AI, a link) goes through `normalize` first (`normalizeDocument`).

## Host bridge

| | standalone | MCP Apps | ChatGPT |
|---|---|---|---|
| detection | no parent frame | iframe, no `window.openai` | `window.openai` |
| document in | `#doc=` hash, empty FA | `ui/notifications/tool-result` → `structuredContent.document` (also `result.document` or the document itself); `tool-input.arguments.document` as a preview | `toolOutput` (same shapes) and `widgetState.document` |
| extras shown (panel "Assistant") | – | `svg`, `analysis`, `simulation`, `verification`, `steps`, anything else | same |
| theme / language | `prefers-color-scheme`, `navigator.languages` | `hostContext.theme`, `hostContext.locale` (+ `host-context-changed`) | `theme`, `locale` |
| "Send to chat" | – | `ui/message` with a short text and the compact JSON | `sendFollowUpMessage` |
| model sees edits | – | `ui/update-model-context` (debounced) | `setWidgetState` |
| size | – | `ui/notifications/size-changed` (ResizeObserver) | `notifyIntrinsicHeight` |
| engine calls | wasm | wasm, or `tools/call` | wasm, or `callTool` |

Handshake order: `ui/initialize` (appInfo, appCapabilities, protocolVersion `2026-01-26`) → wait for the answer →
`ui/notifications/initialized`. If the host never answers (a plain iframe) the page behaves as standalone.
Downloads and the clipboard are usually blocked in host iframes: Save/Export/Copy fall back to a dialog with the text.

## Panel registry (for T12b)

```js
import { registerPanel, registerAction } from './registry.js';
import { registerStrings, t } from './i18n.js';

registerStrings({
  en: { 'grammar.title': 'Grammar' },
  es: { 'grammar.title': 'Gramática' },
});

registerPanel({
  id: 'grammar-editor',          // unique
  title: 'grammar.title',        // an i18n key or plain text
  appliesTo: ['grammar'],        // document types; ['*'] = all; ['automata'] = fa, pda, turing, mealy, moore
  order: 10,                     // optional; lower first (built-ins use 5..90)
  visible(ctx) { return true; }, // optional; hide the panel for tabs where it makes no sense
  mount(container, ctx) {        // once per tab, the first time the panel is shown
    // build DOM into `container` (see dom.js `h`)
    const off = ctx.on('doc', () => redraw());
    return { destroy() { off(); }, focus() {} };   // all optional
  },
});

registerAction({
  id: 'convert.cnf.grammar',
  menu: 'convert',               // file edit input test convert layout view help (or a new id, shown before Help; its title is the i18n key `menu.<id>` or the id itself)
  label: 'convert.cnf',          // i18n key or text
  appliesTo: ['grammar'],
  order: 15, separator: false, shortcut: 'Ctrl+Shift+N',
  enabled(ctx) { return true; }, checked(ctx) { return false; },
  run(ctx) { /* ... */ },
});
```

Import the new module from `main.js` (`import './grammar-panel.js';`); that is all the shell needs. Documents of a
type without a graph (`grammar`, `regex`, `lsystem`, `*_pumping_lemma`) get a full-width side area (no canvas) and
the fallback panel "Document"; automata get the canvas plus the panels.

### `ctx` (a `Tab`, `tab.js`)

| member | meaning |
|---|---|
| `ctx.doc`, `ctx.type`, `ctx.title`, `ctx.path` | the FULL document, its type, the tab title, the state ids walked into building blocks |
| `await ctx.api(request)` | any `rflap-api` op; throws `ApiError` (`.code`, `.message`) |
| `await ctx.edit((clone, path) => { ... })` | applies a change to a clone and commits it (undo history, redraw); toasts and returns `false` if it throws |
| `await ctx.commit(nextDoc)` / `ctx.replace(doc)` | commit a document you built (undoable) / replace without history |
| `ctx.undo()`, `ctx.redo()`, `ctx.history` | undo/redo (50 levels) |
| `ctx.on(event, fn) -> off` | events: `doc` (`{cause}`: `edit`, `undo`, `redo`, `replace`, `host`, `path`, `flags`), `selection`, `overlay`, `path`, `flags`, `steps`, `extras`, `theme`, `activate` |
| `ctx.setOverlay(source, {states: [ids], transitions: [indices]} \| null)` | highlight on the canvas; sources are independent (`'sim'`, `'steps'`, yours) |
| `ctx.steps`, `ctx.setSteps({title, steps, source})` | conversion steps for the step viewer; `source` = the tab converted from (ids in the steps refer to it) |
| `ctx.extras`, `ctx.setExtras(obj)` | what the assistant sent (panel "Assistant") |
| `ctx.openTab(doc, {title, steps, extras})` | open a document (any form after `normalizeDocument`) in a new tab |
| `ctx.notify(message, kind)` | toast (`'info'` or `'error'`) |
| `ctx.app` | the shell: `tabs()`, `openTab`, `closeTab`, `showPanel(tab, id, method?)`, `host`, `dark`, `openText(text, title)`, `sendToChat(tab)` |
| `ctx.host` | host bridge (`mode`, `connected`, `sendMessage`, `callTool`, ...) |

Reusable pieces: `convertToTab(ctx, to, i18nKey)` (actions.js: runs `convert`, opens the result with its steps),
`stepView`, `collectStateIds`, `resolveStep` (steps.js), `valueView(json)` (assistant.js: readable JSON),
`configTable`, `tapeView`, `simSettings`, `runArguments` (sim.js), `askLabel` (labels.js), `formDialog`,
`messageDialog`, `popupMenu`, `toast` (dialogs.js), and everything in `doc.js`. `window.Rflap` exposes
`{app, api, registerPanel, registerAction, registerStrings}` for experiments in the console.

## Keyboard

Canvas focused: `1 2 3 4` tools, `Del` delete, `F2`/`Enter` rename or edit, arrows move (Shift = 1 px), `+ − 0` zoom,
`Shift+F10` context menu, `Esc` clear selection. Everywhere: `Ctrl+Z`, `Ctrl+Y`/`Ctrl+Shift+Z`, `Ctrl+O`, `Ctrl+S`,
`Ctrl+A`, `Ctrl+W`, `Ctrl+= / − / 0`. The "Structure" panel offers everything the mouse does as plain tables and
forms (keyboard and screen readers).

## Not covered here (T12b)

Panels for grammars, regular expressions, step-by-step conversion replay with richer views, parsing (brute force,
LL, SLR, CYK, user parse), design from a language, L-systems and the pumping lemma. Those documents already open
(from `.jff` or from the assistant) in the "Document" panel and can be converted from the Convert menu.
