import test from 'node:test';
import assert from 'node:assert/strict';
import { createHost, detectMode, MCP_PROTOCOL_VERSION } from '../src/host.js';

/** A window whose parent records what the view posts and can deliver host messages back. */
function fakeWindow(extra = {}) {
  const parent = { sent: [], postMessage(message, origin) { this.sent.push({ message, origin }); } };
  const listeners = {};
  const win = {
    parent,
    addEventListener(type, fn) { (listeners[type] ||= []).push(fn); },
    deliver(data, source = parent) { (listeners.message || []).forEach((fn) => fn({ data, source })); },
    dispatch(type, event) { (listeners[type] || []).forEach((fn) => fn(event)); },
    ...extra,
  };
  win.sentMessages = () => parent.sent.map((s) => s.message);
  return win;
}

const INIT_RESULT = {
  protocolVersion: MCP_PROTOCOL_VERSION,
  hostCapabilities: { serverTools: {}, openLinks: {} },
  hostInfo: { name: 'fake', version: '1' },
  hostContext: { theme: 'dark', locale: 'es-AR' },
};

/** Connects an MCP host, answering ui/initialize like a real host. */
async function connected(handlers = {}) {
  const win = fakeWindow();
  const host = createHost(win, handlers, { timeoutMs: 500 });
  const connecting = host.connect();
  const [init] = win.sentMessages();
  win.deliver({ jsonrpc: '2.0', id: init.id, result: INIT_RESULT });
  await connecting;
  return { win, host };
}

const tick = () => new Promise((r) => setTimeout(r, 0));

test('mode detection', () => {
  const alone = { parent: null };
  alone.parent = alone;
  assert.equal(detectMode(alone), 'standalone');
  assert.equal(detectMode(fakeWindow()), 'mcp');
  assert.equal(detectMode(fakeWindow({ openai: {} })), 'chatgpt');
});

test('handshake order: ui/initialize request, then ui/notifications/initialized after the answer', async () => {
  const win = fakeWindow();
  const host = createHost(win, {});
  const connecting = host.connect();
  assert.equal(win.sentMessages().length, 1);
  const [init] = win.sentMessages();
  assert.equal(init.jsonrpc, '2.0');
  assert.equal(init.method, 'ui/initialize');
  assert.equal(init.params.protocolVersion, '2026-01-26');
  assert.equal(init.params.appInfo.name, 'rflap');
  assert.ok(init.params.appCapabilities);
  assert.equal(host.connected, false);
  win.deliver({ jsonrpc: '2.0', id: init.id, result: INIT_RESULT });
  await connecting;
  const [, initialized] = win.sentMessages();
  assert.equal(initialized.method, 'ui/notifications/initialized');
  assert.equal(initialized.id, undefined);
  assert.equal(host.connected, true);
  assert.equal(host.canCallTools, true);
  assert.equal(host.context.theme, 'dark');
  assert.equal(win.parent.sent[0].origin, '*');
});

test('the handshake fails (and nothing else is sent) when the host never answers', async () => {
  const win = fakeWindow();
  const host = createHost(win, {}, { timeoutMs: 20 });
  await assert.rejects(host.connect(), /did not answer/);
  assert.equal(win.sentMessages().length, 1);
  assert.equal(host.connected, false);
});

test('tool-input and tool-result reach the handlers; structuredContent carries the document', async () => {
  const seen = [];
  const { win } = await connected({
    onToolInput: (args) => seen.push(['input', args]),
    onToolResult: (result) => seen.push(['result', result]),
  });
  win.deliver({ jsonrpc: '2.0', method: 'ui/notifications/tool-input', params: { arguments: { document: { type: 'fa' } } } });
  const document = { type: 'fa', initial: 'q0', transitions: [] };
  win.deliver({ jsonrpc: '2.0', method: 'ui/notifications/tool-result', params: { content: [{ type: 'text', text: 'ok' }], structuredContent: { document } } });
  assert.deepEqual(seen[0], ['input', { document: { type: 'fa' } }]);
  assert.equal(seen[1][0], 'result');
  assert.deepEqual(seen[1][1].structuredContent.document, document);
});

test('host-context-changed merges into the context and notifies', async () => {
  const contexts = [];
  const { win, host } = await connected({ onContext: (c) => contexts.push(c) });
  win.deliver({ jsonrpc: '2.0', method: 'ui/notifications/host-context-changed', params: { theme: 'light' } });
  assert.equal(host.context.theme, 'light');
  assert.equal(host.context.locale, 'es-AR');
  assert.equal(contexts.at(-1).theme, 'light');
});

test('ui/message sends a user text message and resolves on the answer', async () => {
  const { win, host } = await connected();
  const sending = host.sendMessage('hello');
  const message = win.sentMessages().at(-1);
  assert.equal(message.method, 'ui/message');
  assert.deepEqual(message.params, { role: 'user', content: { type: 'text', text: 'hello' } });
  win.deliver({ jsonrpc: '2.0', id: message.id, result: {} });
  assert.equal(await sending, true);
});

test('a host error rejects the request', async () => {
  const { win, host } = await connected();
  const sending = host.sendMessage('x');
  win.deliver({ jsonrpc: '2.0', id: win.sentMessages().at(-1).id, error: { code: -32000, message: 'denied' } });
  await assert.rejects(sending, /denied/);
});

test('ui/update-model-context carries text and structured content', async () => {
  const { win, host } = await connected();
  const updating = host.updateModelContext({ text: 'fa: 2 states', structured: { document: { type: 'fa' } } });
  const message = win.sentMessages().at(-1);
  assert.equal(message.method, 'ui/update-model-context');
  assert.deepEqual(message.params, { content: [{ type: 'text', text: 'fa: 2 states' }], structuredContent: { document: { type: 'fa' } } });
  win.deliver({ jsonrpc: '2.0', id: message.id, result: {} });
  await updating;
});

test('tools/call goes through the host and returns the result', async () => {
  const { win, host } = await connected();
  const calling = host.callTool('rflap_call', { request: '{}' });
  const message = win.sentMessages().at(-1);
  assert.equal(message.method, 'tools/call');
  assert.deepEqual(message.params, { name: 'rflap_call', arguments: { request: '{}' } });
  win.deliver({ jsonrpc: '2.0', id: message.id, result: { structuredContent: { ok: true } } });
  assert.deepEqual(await calling, { structuredContent: { ok: true } });
});

test('size-changed and open-link', async () => {
  const { win, host } = await connected();
  host.notifySize(400, 300);
  assert.deepEqual(win.sentMessages().at(-1), { jsonrpc: '2.0', method: 'ui/notifications/size-changed', params: { width: 400, height: 300 } });
  const opening = host.openLink('https://example.test');
  const message = win.sentMessages().at(-1);
  assert.equal(message.method, 'ui/open-link');
  win.deliver({ jsonrpc: '2.0', id: message.id, result: {} });
  await opening;
});

test('ui/resource-teardown is a request: the handler runs and the host gets an empty result', async () => {
  let reason = null;
  const { win } = await connected({ onTeardown: (params) => (reason = params.reason) });
  win.deliver({ jsonrpc: '2.0', id: 77, method: 'ui/resource-teardown', params: { reason: 'closing' } });
  await tick();
  assert.equal(reason, 'closing');
  assert.deepEqual(win.sentMessages().at(-1), { jsonrpc: '2.0', id: 77, result: {} });
});

test('unknown host requests get a method-not-found error; foreign sources and junk are ignored', async () => {
  const seen = [];
  const { win } = await connected({ onToolResult: (r) => seen.push(r) });
  win.deliver({ jsonrpc: '2.0', id: 5, method: 'something/else' });
  await tick();
  assert.equal(win.sentMessages().at(-1).error.code, -32601);
  const before = win.sentMessages().length;
  win.deliver({ jsonrpc: '2.0', method: 'ui/notifications/tool-result', params: {} }, { other: true });
  win.deliver('not an object');
  win.deliver({ method: 'ui/notifications/tool-result', params: {} });
  assert.equal(seen.length, 0);
  assert.equal(win.sentMessages().length, before);
});

test('standalone: everything is a harmless no-op', async () => {
  const host = createHost({ parent: null }, {}, { mode: 'standalone' });
  await host.connect();
  assert.equal(await host.sendMessage('x'), false);
  assert.equal(await host.updateModelContext({ text: 'x' }), false);
  assert.equal(host.connected, false);
  host.notifySize(1, 1);
  await assert.rejects(host.callTool('x', {}));
});

test('ChatGPT: toolOutput becomes a tool result; follow-up message, widget state, callTool and set_globals', async () => {
  const calls = [];
  const openai = {
    toolInput: { q: 1 },
    toolOutput: { document: { type: 'fa' } },
    widgetState: { document: { type: 'pda' } },
    theme: 'dark',
    locale: 'es',
    sendFollowUpMessage: async (m) => calls.push(['follow', m]),
    setWidgetState: async (s) => calls.push(['state', s]),
    callTool: async (name, args) => ({ result: JSON.stringify({ ok: true, name, args }) }),
    notifyIntrinsicHeight: (h) => calls.push(['height', h]),
  };
  const win = fakeWindow({ openai });
  const seen = [];
  const host = createHost(win, { onToolInput: (a) => seen.push(['input', a]), onToolResult: (r) => seen.push(['result', r]), onContext: (c) => seen.push(['context', c]) });
  assert.equal(host.mode, 'chatgpt');
  await host.connect();
  assert.deepEqual(seen.find((s) => s[0] === 'input')[1], { q: 1 });
  const result = seen.find((s) => s[0] === 'result')[1];
  assert.deepEqual(result.structuredContent, { document: { type: 'fa' } });
  assert.deepEqual(result.widgetState, { document: { type: 'pda' } });
  assert.equal(seen.find((s) => s[0] === 'context')[1].theme, 'dark');
  await host.sendMessage('hi');
  await host.updateModelContext({ text: 'summary', structured: { document: { type: 'fa' } } });
  host.notifySize(100, 222);
  assert.deepEqual(calls, [['follow', { prompt: 'hi' }], ['state', { document: { type: 'fa' }, summary: 'summary' }], ['height', 222]]);
  const answer = await host.callTool('t', { a: 1 });
  assert.deepEqual(answer.structuredContent, { ok: true, name: 't', args: { a: 1 } });
  win.dispatch('openai:set_globals', { detail: { globals: { toolOutput: { document: { type: 'moore' } } } } });
  assert.deepEqual(seen.filter((s) => s[0] === 'result').at(-1)[1].structuredContent, { document: { type: 'moore' } });
});
