// Entry point: wires the engine (wasm, or the MCP server as a fallback), the host bridge and the app shell.
// The panels and actions register themselves when their modules are imported.

import './sim.js';
import './structure.js';
import './analysis.js';
import './steps.js';
import './assistant.js';
import './info.js';
import './actions.js';
import { App } from './app.js';
import { createApi, localTransport, remoteTransport } from './api.js';
import { embeddedWasmBytes, loadWasm } from './wasm.js';
import { createHost } from './host.js';
import { registerPanel, registerAction } from './registry.js';
import { registerStrings, t } from './i18n.js';
import { emptyDocument } from './doc.js';
import { debounce } from './dom.js';

async function boot() {
  let transport = null;
  const api = createApi(() => transport);
  let ready;
  const whenReady = new Promise((resolve) => (ready = resolve));
  let app = null;

  const host = createHost(window, {
    onToolInput: (args) => whenReady.then(() => args.document && app.showToolResult({ document: args.document })),
    onToolResult: (result) =>
      whenReady.then(() => {
        if (result && !result.isError) {
          const saved = result.widgetState && result.widgetState.document;
          return app.showToolResult(result.structuredContent, { saved });
        }
        return undefined;
      }),
    onContext: (context) => app && app.handleContext(context),
  });
  if (host.mode !== 'standalone') document.documentElement.dataset.embedded = '';

  app = new App(document.getElementById('app'), { api, host });
  app.mount();
  window.Rflap = { app, registerPanel, registerAction, registerStrings, api };

  try {
    await host.connect();
    host.connected = host.mode === 'chatgpt' || host.mode === 'mcp';
  } catch (error) {
    host.connected = false; // opened in a plain iframe: behave as standalone
  }
  app.handleContext(host.context || {});

  try {
    transport = localTransport(await loadWasm(embeddedWasmBytes(document)));
  } catch (wasmError) {
    // MCP Apps' default CSP has no 'wasm-unsafe-eval': ask the MCP server for the engine instead
    if (host.connected && host.canCallTools) {
      transport = remoteTransport(host, window.RFLAP_REMOTE_TOOL || 'rflap_call');
      app.notify(t('remote.fallback'), 'info');
    } else {
      app.showFatal(`${wasmError.message}`);
      return;
    }
  }
  ready();

  if (host.mode === 'standalone' || !host.connected) {
    if (!(await app.openFromHash())) app.openTab(emptyDocument('fa'));
  } else if (!app.tabs().length) {
    app.openTab(emptyDocument('fa'));
  }

  if (typeof ResizeObserver === 'function') {
    const appEl = document.getElementById('app');
    // the height of the app itself (not of the viewport), so the host can also shrink the frame
    const report = debounce(() => host.notifySize(document.documentElement.clientWidth, Math.ceil(appEl.getBoundingClientRect().height)), 100);
    new ResizeObserver(report).observe(document.documentElement);
    new ResizeObserver(report).observe(appEl);
    report();
  }
}

boot();
