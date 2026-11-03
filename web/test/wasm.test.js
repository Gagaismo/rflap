import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { decodeBase64, loadWasm, makeWasmCaller } from '../src/wasm.js';
import { createApi, localTransport, ApiError } from '../src/api.js';

const dist = path.join(path.dirname(fileURLToPath(import.meta.url)), '../dist/rflap.html');
const html = fs.readFileSync(dist, 'utf8');
const base64 = /<script id="rflap-wasm" type="application\/octet-stream">([^<]+)<\/script>/.exec(html)[1];
const bytes = decodeBase64(base64);
const { instance } = await WebAssembly.instantiate(bytes, {});
const wasmExports = instance.exports;

test('decodeBase64 agrees with Node for every padding length', () => {
  for (const n of [0, 1, 2, 3, 4, 5, 255, 256, 1000]) {
    const data = Uint8Array.from({ length: n }, (_, i) => (i * 37 + 11) & 255);
    assert.deepEqual(decodeBase64(Buffer.from(data).toString('base64')), data, `length ${n}`);
  }
  assert.deepEqual(decodeBase64(Buffer.from('line\nbreaks ok').toString('base64').replace(/(.{4})/g, '$1\n')), new TextEncoder().encode('line\nbreaks ok'));
});

test('the page embeds a real wasm module (the one in dist/rflap.html)', async () => {
  assert.deepEqual([...bytes.subarray(0, 4)], [0, 0x61, 0x73, 0x6d]);
  const module = await WebAssembly.compile(bytes);
  const exports = WebAssembly.Module.exports(module).map((e) => e.name);
  for (const name of ['memory', 'rflap_alloc', 'rflap_free', 'rflap_call', 'rflap_free_result']) assert.ok(exports.includes(name), name);
  assert.deepEqual(WebAssembly.Module.imports(module), [], 'the module needs no imports (no wasm-bindgen glue)');
});

test('capabilities and simulate run in the built wasm', async () => {
  const call = await loadWasm(bytes);
  const capabilities = JSON.parse(call('{"op":"capabilities"}'));
  assert.equal(capabilities.ok, true);
  assert.ok(capabilities.result.ops.length > 10);
  const simulate = JSON.parse(
    call(JSON.stringify({ op: 'simulate', input: 'ab', document: { type: 'fa', initial: 'q0', finals: ['q2'], transitions: [['q0', 'a', 'q1'], ['q1', 'b', 'q2']] } })),
  );
  assert.equal(simulate.ok, true);
  assert.equal(simulate.result.outcome, 'accepted');
  assert.deepEqual(simulate.result.trace.map((c) => c.state), ['q0', 'q1', 'q2']);
});

test('the wasm answers errors, non-ASCII text and many calls (memory is freed)', async () => {
  const call = await loadWasm(bytes);
  assert.equal(JSON.parse(call('not json')).error.code, 'invalid_json');
  assert.equal(JSON.parse(call('{"op":"nope"}')).error.code, 'unknown_op');
  const answer = JSON.parse(call(JSON.stringify({ op: 'simulate', input: 'λ', document: { type: 'fa', initial: 'q0', finals: ['q0'], transitions: [['q0', 'λ', 'q0']] } })));
  assert.equal(answer.ok, true);
  const request = JSON.stringify({ op: 'render', document: { type: 'fa', initial: 'q0', transitions: [['q0', 'a', 'q0']] }, format: 'svg' });
  for (let i = 0; i < 300; i++) assert.equal(JSON.parse(call(request)).ok, true);
});

test('api(): results unwrap, errors become ApiError with the engine code', async () => {
  const api = createApi(() => localTransport(makeWasmCaller(wasmExports)));
  const result = await api({ op: 'normalize', document: { type: 'fa', initial: 'q0', finals: ['q0'], transitions: [['q0', 'a', 'q0']] } });
  assert.equal(result.document.type, 'fa');
  assert.equal(Object.keys(result.document.states).length, 1);
  await assert.rejects(api({ op: 'normalize', document: { type: 'pda', initial: 'a', transitions: [['a', 'xx', 'a']] } }), (e) => e instanceof ApiError && e.code === 'invalid_label');
  const none = createApi(() => null);
  await assert.rejects(none({ op: 'capabilities' }), /not loaded/);
});

test('the full document form comes back from normalize with the shapes doc.js expects', async () => {
  const api = createApi(() => localTransport(makeWasmCaller(wasmExports)));
  const { document: tm } = await api({ op: 'normalize', document: { type: 'turing', initial: 'q0', finals: ['q1'], transitions: [['q0', 'a;b,R|_;x,S', 'q1']] } });
  assert.equal(tm.tapes, 2);
  assert.deepEqual(tm.automaton.transitions[0].label, { actions: [{ direction: 'right', read: 'a', write: 'b' }, { direction: 'stay', read: '□', write: 'x' }] });
  const { document: pda } = await api({ op: 'normalize', document: { type: 'pda', initial: 'q0', transitions: [['q0', 'a,Z;AZ', 'q0']] } });
  assert.deepEqual(pda.transitions[0].label, { input: 'a', pop: 'Z', push: 'AZ' });
  const { document: blank } = await api({ op: 'normalize', document: { type: 'turing', tapes: 3 } });
  assert.equal(blank.tapes, 3);
});
