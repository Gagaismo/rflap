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
| `index.html`, `style.css` | template (the xtask fills three markers) and styles (the classic JFLAP look only, responsive) |
| `main.js` | entry: host, engine (wasm or remote), app shell |
| `wasm.js` | wasm loader: `decodeBase64`, `loadWasm(bytes) -> call(text) -> text` over the C ABI of `rflap-wasm` |
| `api.js` | `createApi(getTransport)` -> `api(request) -> result` (throws `ApiError` with the engine's `code`); `localTransport`, `remoteTransport` |
| `doc.js` | helpers over the FULL document JSON (add/remove state/transition, blocks, labels, compact form, `History`) |
| `share.js` | `#doc=<base64url JSON>` links |
| `host.js` | host bridge: `createHost(window, handlers)` for standalone, MCP Apps and ChatGPT |
| `i18n.js` | Spanish (default) and English; `t(key, params)`, `registerStrings`, `pickLocale` |
| `fit.js` | pure "fit to window": `fitView(box, viewport, options) -> {scale, offsetX, offsetY}` |
| `registry.js` | **panel and action registries** (below) |
| `tab.js` | `Tab`: a document with undo history, flags, overlays; it is the `ctx` of panels and actions |
| `app.js` | shell: menu bar, tabs, workspaces, files, host integration |
| `editor.js` | the canvas (engine SVG made interactive through `data-state-id`, `data-transition-index`, ...) |
| `labels.js`, `dialogs.js`, `dom.js` | label forms per type, modals/menus/toasts, DOM helpers |
| `sim.js`, `structure.js`, `analysis.js`, `steps.js`, `assistant.js`, `info.js` | built-in panels (`steps.js` is the conversion replay, see "Step replay") |
| `actions.js` | File / Edit / Input / Test / Convert / Layout / View / Help menu actions |
| `grammar-editor.js`, `grammar-transform.js`, `grammar-tables.js`, `grammar-parse.js`, `grammar-model.js` | grammar environment: production table + type test, transformations replay, FIRST/FOLLOW/LL(1)/SLR(1) tables, parsers, user parse; `grammar-model.js` is the pure part |
| `regex-panel.js` | regular expression environment: editor with validation, conversions, equivalence |
| `design-panel.js`, `design-model.js` | design from a language (S4): form, report, pure form -> request mapping |
| `lsystem-panel.js`, `lsystem-model.js` | L-system environment |
| `pumping-panel.js`, `pumping-model.js` | pumping lemma game (regular and context-free) |
| `replay.js`, `replay-model.js`, `replay-views.js` | step replay: the state machine and control bar, the pure frames of each conversion, the richer views |
| `tree.js`, `richtext.js`, `widgets.js` | parse tree layout and SVG, the HTML subset of the pumping texts, shared DOM pieces |

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
| theme / language | ignored: always the classic look, Spanish by default | `hostContext.theme` and `locale` are ignored | `theme` and `locale` are ignored; the chosen language is kept in `widgetState.language` |
| "Send to chat" | – | `ui/message` with a short text and the compact JSON | `sendFollowUpMessage` |
| model sees edits | – | `ui/update-model-context` (debounced) | `setWidgetState` |
| size | – | `ui/notifications/size-changed` (ResizeObserver) | `notifyIntrinsicHeight` |
| engine calls | wasm | wasm, or `tools/call` | wasm, or `callTool` |

Handshake order: `ui/initialize` (appInfo, appCapabilities, protocolVersion `2026-01-26`) → wait for the answer →
`ui/notifications/initialized`. If the host never answers (a plain iframe) the page behaves as standalone.
Downloads and the clipboard are usually blocked in host iframes: Save/Export/Copy fall back to a dialog with the text.

## Panel registry

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
| `ctx.app` | the shell: `tabs()`, `openTab`, `closeTab`, `showPanel(tab, id, method?)`, `host`, `languageChoice`, `openText(text, title)`, `sendToChat(tab)` |
| `ctx.host` | host bridge (`mode`, `connected`, `sendMessage`, `callTool`, ...) |

Reusable pieces: `convertToTab(ctx, to, i18nKey)` (actions.js: runs `convert`, opens the result with its steps),
`stepView`, `collectStateIds`, `resolveStep` (steps.js), `valueView(json)` (assistant.js: readable JSON),
`configTable`, `tapeView`, `simSettings`, `runArguments` (sim.js), `askLabel` (labels.js), `formDialog`,
`messageDialog`, `popupMenu`, `toast` (dialogs.js), and everything in `doc.js`. `window.Rflap` exposes
`{app, api, registerPanel, registerAction, registerStrings}` for experiments in the console.

## Keyboard

Canvas focused: `1 2 3 4` tools, `Del` delete, `F2`/`Enter` rename or edit, arrows move (Shift = 1 px), `+ − 0` zoom, `9` fit,
`Shift+F10` context menu, `Esc` clear selection. Everywhere: `Ctrl+Z`, `Ctrl+Y`/`Ctrl+Shift+Z`, `Ctrl+O`, `Ctrl+S`,
`Ctrl+A`, `Ctrl+W`, `Ctrl+= / − / 0` zoom, `Ctrl+9` fit to window. The "Structure" panel offers everything the mouse does as plain tables and
forms (keyboard and screen readers).

## Look, zoom and language

* **One look.** The editor, the panels and the assistant image always use the classic JFLAP style (white canvas,
  yellow states, black strokes). The host theme (`hostContext.theme`, ChatGPT's `theme`) and
  `prefers-color-scheme` are ignored on purpose; there is no dark mode and `render` is always called without `dark`.
* **Fit.** A document that is loaded (tool result, file, link, new tab, conversion result) is fitted when it is drawn
  and again when the canvas first gets a real size or resizes (iframes start small), until the user zooms, clicks or
  edits. The fit shows the whole drawing (measured with `getBBox`: circles, labels, loops, initial triangle, notes)
  with a margin and never above 100%. The "Ajustar" button, View > Fit to window and `Ctrl+9` / `9` do it on demand.
  The arithmetic is `fitView` in `fit.js` (tested).
* **Language.** Always Spanish at start; `navigator.language` and the host locale are ignored. View > Language
  switches to English; the choice is kept in `localStorage` (`rflap.language`) and, in ChatGPT, in
  `widgetState.language`. Every string must exist in both dictionaries (`web/test/i18n.test.js`).

## Environments (T12b)

Every JFLAP 7.1 environment opens in a tab with its own panels (the panel ids are in parentheses); the File menu
has a "New ..." entry for each, and `.jff` files, links and the assistant open them directly.

| document | panels | what they do |
|---|---|---|
| `grammar` | Grammar (`grammar-editor`) | production table (add, remove, reorder, λ button), start variable, paste `S -> aSb \| λ` text, live type test (`grammar_type`) |
| | Transform (`grammar-transform`) | remove λ / unit / useless productions and Chomsky normal form, replayed step by step in JFLAP's order (λ, unit, useless, then CNF); open the result in a tab or replace the grammar |
| | Tables (`grammar-tables`) | FIRST/FOLLOW, LL(1) table, SLR(1) items, ACTION and GOTO from `parse_tables`; conflicts are highlighted and listed |
| | Parse (`grammar-parse`) | brute force, LL(1) and SLR(1) traces (step through them), CYK triangle, parse tree as SVG and the derivation, one row per input for several strings |
| | User parse (`grammar-user`) | pick the production and the occurrence to expand (`user_parse`), undo, tree of the partial derivation |
| `regex` | Expression (`regex-editor`) | editor with live validation (the error position is marked), convert to NFA / DFA / minimal DFA, compare with another tab (`equivalent`) |
| `lsystem` | L-system (`lsystem-editor`) | axiom, rules and parameters, depth slider, drawing from `lsystem_render`, SVG export |
| `regular_pumping_lemma`, `context_free_pumping_lemma` | Pumping lemma (`pumping-game`) | language chooser (the 26 of `pumping_languages`), who goes first, the boxes in JFLAP order, decomposition sliders, result and explanation, cases panel of the context-free "you go first" game; the tab's document is the game state, so undo steps back a move |
| any | Design (`design`) | alphabet, constraint / regular expression / regular grammar / template, accept and reject examples, `design`; states with their meaning, regex, verification report (failed examples in red), open as a tab. File > New automaton from a language... shows it |

The grammar conversions to a regular grammar -> FA and to a PDA (LL, LR) are in the Convert menu and open a new tab.

### Step replay

`convertToTab` (actions.js) hands the step viewer `{title, steps, source, to, tree, sourceDoc}`. The "Steps" panel
steps through them with `Replay` (replay.js: next / previous / first / do all / play, boundaries answer whether
they moved). Keys inside the panel: `n` or `→` next, `p` or `←` previous, `Home`, `End` or `a` do all, `Space` play.
`replay-model.js` turns the steps into one frame per step (pure, tested against recorded engine answers) and
`replay-views.js` draws them:

* NFA -> DFA: the DFA states built so far with the NFA states they stand for (the states of the step are also
  highlighted on both tabs, as before).
* Minimization: the partition after each split and the distinguishability tree growing split by split.
* FA -> RE: the generalized transition graph after each step (the engine's `render` draws it), with its labels.
* RE -> NFA: the graph after each de-expressionify step, the new states highlighted.

Anything else keeps the generic step view.

## Tests

`web/test/*.test.js` runs with `node --test`. The model tests use small recorded answers of the real engine
(`web/test/fixtures/*.json`, produced by `node web/test/fixtures/record.mjs`, which loads the wasm of
`dist/rflap.html` through `web/test/engine.js`); a few tests also call the engine live, so build first.
