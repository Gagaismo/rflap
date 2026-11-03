// Host bridge: the same page runs standalone, as an MCP Apps view (Claude, ...) and as a ChatGPT widget.
//
//   standalone : no host; every host call is a no-op / resolves to false.
//   mcp        : JSON-RPC 2.0 over postMessage with window.parent (spec 2026-01-26, SEP-1865).
//                View -> host: ui/initialize, ui/notifications/initialized, tools/call, ui/message,
//                ui/update-model-context, ui/open-link, ui/notifications/size-changed.
//                Host -> view: ui/notifications/tool-input, tool-result, host-context-changed, ui/resource-teardown.
//   chatgpt    : window.openai (toolOutput, callTool, sendFollowUpMessage, setWidgetState, theme,
//                the `openai:set_globals` event).
//
// `createHost(win, handlers)` returns one object with the same methods in the three modes:
//   mode, context, connect(), canCallTools, sendMessage(text), updateModelContext({text, structured}),
//   callTool(name, args), openLink(url), notifySize(width, height)
// handlers: { onToolInput(args), onToolResult(callToolResult), onContext(context), onTeardown() }

export const MCP_PROTOCOL_VERSION = '2026-01-26';
export const APP_INFO = { name: 'rflap', version: '0.1.0' };

export function detectMode(win) {
  if (win.openai) return 'chatgpt';
  if (win.parent && win.parent !== win) return 'mcp';
  return 'standalone';
}

export function createHost(win, handlers = {}, options = {}) {
  const mode = options.mode || detectMode(win);
  if (mode === 'mcp') return new McpHost(win, handlers, options);
  if (mode === 'chatgpt') return new ChatGptHost(win, handlers);
  return new StandaloneHost(handlers);
}

const noop = () => {};

class StandaloneHost {
  constructor(handlers) {
    this.mode = 'standalone';
    this.handlers = handlers;
    this.context = {};
    this.canCallTools = false;
    this.connected = false;
  }

  async connect() {}

  async sendMessage() {
    return false;
  }

  async updateModelContext() {
    return false;
  }

  async callTool() {
    throw new Error('there is no host to call tools on');
  }

  async openLink(url) {
    if (typeof globalThis.open === 'function') globalThis.open(url, '_blank', 'noopener');
    return true;
  }

  notifySize() {}
}

class McpHost {
  constructor(win, handlers, options) {
    this.mode = 'mcp';
    this.win = win;
    this.handlers = handlers;
    this.timeoutMs = options.timeoutMs ?? 4000;
    this.context = {};
    this.connected = false;
    this.hostCapabilities = {};
    this.nextId = 1;
    this.pending = new Map();
    this.listening = false;
  }

  get canCallTools() {
    return Boolean(this.hostCapabilities.serverTools);
  }

  listen() {
    if (this.listening) return;
    this.listening = true;
    this.win.addEventListener('message', (event) => this.receive(event));
  }

  post(message) {
    this.win.parent.postMessage({ jsonrpc: '2.0', ...message }, '*');
  }

  request(method, params) {
    const id = this.nextId++;
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
      this.post({ id, method, params });
    });
  }

  notify(method, params) {
    this.post({ method, params });
  }

  receive(event) {
    if (event.source !== undefined && event.source !== this.win.parent) return;
    const message = event.data;
    if (!message || message.jsonrpc !== '2.0') return;
    if (typeof message.method === 'string') {
      if (message.id !== undefined) this.answerRequest(message);
      else this.dispatchNotification(message.method, message.params || {});
    } else if (message.id !== undefined && this.pending.has(message.id)) {
      const { resolve, reject } = this.pending.get(message.id);
      this.pending.delete(message.id);
      if (message.error) reject(new Error(message.error.message || 'host error'));
      else resolve(message.result);
    }
  }

  async answerRequest(message) {
    try {
      if (message.method === 'ui/resource-teardown') {
        await (this.handlers.onTeardown || noop)(message.params || {});
      } else if (message.method !== 'ping') {
        this.post({ id: message.id, error: { code: -32601, message: `unsupported request ${message.method}` } });
        return;
      }
      this.post({ id: message.id, result: {} });
    } catch (error) {
      this.post({ id: message.id, error: { code: -32000, message: String(error && error.message) } });
    }
  }

  dispatchNotification(method, params) {
    switch (method) {
      case 'ui/notifications/tool-input':
        (this.handlers.onToolInput || noop)(params.arguments || {});
        break;
      case 'ui/notifications/tool-result':
        (this.handlers.onToolResult || noop)(params);
        break;
      case 'ui/notifications/host-context-changed':
        this.context = { ...this.context, ...params };
        (this.handlers.onContext || noop)(this.context);
        break;
      default:
        break; // tool-input-partial, tool-cancelled, ... are not needed
    }
  }

  /** Handshake: ui/initialize, then ui/notifications/initialized. Rejects if the host does not answer. */
  async connect() {
    this.listen();
    let timer;
    const timeout = new Promise((_, reject) => {
      timer = setTimeout(() => reject(new Error('the host did not answer ui/initialize')), this.timeoutMs);
    });
    let result;
    try {
      result = await Promise.race([
        this.request('ui/initialize', {
          appInfo: APP_INFO,
          appCapabilities: { availableDisplayModes: ['inline'] },
          protocolVersion: MCP_PROTOCOL_VERSION,
        }),
        timeout,
      ]);
    } finally {
      clearTimeout(timer);
    }
    this.hostCapabilities = (result && result.hostCapabilities) || {};
    this.context = (result && result.hostContext) || {};
    (this.handlers.onContext || noop)(this.context);
    this.notify('ui/notifications/initialized', {});
    this.connected = true;
  }

  async sendMessage(text) {
    await this.request('ui/message', { role: 'user', content: { type: 'text', text } });
    return true;
  }

  async updateModelContext({ text, structured }) {
    const params = {};
    if (text) params.content = [{ type: 'text', text }];
    if (structured) params.structuredContent = structured;
    await this.request('ui/update-model-context', params);
    return true;
  }

  callTool(name, args) {
    return this.request('tools/call', { name, arguments: args });
  }

  async openLink(url) {
    await this.request('ui/open-link', { url });
    return true;
  }

  notifySize(width, height) {
    this.notify('ui/notifications/size-changed', { width, height });
  }
}

class ChatGptHost {
  constructor(win, handlers) {
    this.mode = 'chatgpt';
    this.win = win;
    this.handlers = handlers;
    this.canCallTools = typeof win.openai.callTool === 'function';
    this.connected = true;
  }

  get openai() {
    return this.win.openai;
  }

  get context() {
    return { theme: this.openai.theme, locale: this.openai.locale, widgetState: this.openai.widgetState };
  }

  async connect() {
    this.win.addEventListener('openai:set_globals', (event) => {
      const globals = (event.detail && event.detail.globals) || {};
      if ('toolOutput' in globals && globals.toolOutput) this.deliverOutput(globals.toolOutput);
      if ('theme' in globals || 'locale' in globals) (this.handlers.onContext || noop)(this.context);
    });
    (this.handlers.onContext || noop)(this.context);
    if (this.openai.toolInput) (this.handlers.onToolInput || noop)(this.openai.toolInput);
    if (this.openai.toolOutput) this.deliverOutput(this.openai.toolOutput);
  }

  deliverOutput(toolOutput) {
    (this.handlers.onToolResult || noop)({
      structuredContent: toolOutput,
      widgetState: this.openai.widgetState || null,
    });
  }

  async sendMessage(text) {
    await this.openai.sendFollowUpMessage({ prompt: text });
    return true;
  }

  async updateModelContext({ text, structured }) {
    if (typeof this.openai.setWidgetState !== 'function') return false;
    await this.openai.setWidgetState({ ...(structured || {}), summary: text });
    return true;
  }

  async callTool(name, args) {
    const result = await this.openai.callTool(name, args);
    if (result && result.structuredContent) return result;
    if (result && typeof result.result === 'string') {
      try {
        return { structuredContent: JSON.parse(result.result), content: [{ type: 'text', text: result.result }] };
      } catch {
        return { content: [{ type: 'text', text: result.result }] };
      }
    }
    return result;
  }

  async openLink(url) {
    if (typeof this.openai.openExternal === 'function') this.openai.openExternal({ href: url });
    return true;
  }

  notifySize(width, height) {
    if (typeof this.openai.notifyIntrinsicHeight === 'function') this.openai.notifyIntrinsicHeight(height);
  }
}
