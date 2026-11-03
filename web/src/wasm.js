// Loader of the rflap-wasm module (raw C ABI, see crates/rflap-wasm/src/lib.rs).
//
// The wasm is embedded in the page as base64 in the element with id rflap-wasm (type application/octet-stream).
// Tests (node) pass the bytes directly. Everything here is synchronous after `loadWasm`.

const B64 = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/';

/** Standard base64 -> bytes (no dependency on atob, which is slow for megabytes). */
export function decodeBase64(text) {
  const clean = text.replace(/[^A-Za-z0-9+/]/g, '');
  const out = new Uint8Array(Math.floor((clean.length * 3) / 4));
  const lookup = new Int16Array(128).fill(-1);
  for (let i = 0; i < 64; i++) lookup[B64.charCodeAt(i)] = i;
  let o = 0;
  for (let i = 0; i < clean.length; i += 4) {
    const a = lookup[clean.charCodeAt(i)];
    const b = lookup[clean.charCodeAt(i + 1)];
    const c = i + 2 < clean.length ? lookup[clean.charCodeAt(i + 2)] : -1;
    const d = i + 3 < clean.length ? lookup[clean.charCodeAt(i + 3)] : -1;
    out[o++] = (a << 2) | (b >> 4);
    if (c >= 0) out[o++] = ((b & 15) << 4) | (c >> 2);
    if (d >= 0) out[o++] = ((c & 3) << 6) | d;
  }
  return out.subarray(0, o);
}

/** Bytes of the wasm embedded in the page. */
export function embeddedWasmBytes(doc) {
  const node = doc.getElementById('rflap-wasm');
  if (!node || !node.textContent.trim()) throw new Error('the page has no embedded wasm');
  return decodeBase64(node.textContent);
}

const internalError = (message) =>
  JSON.stringify({ ok: false, error: { code: 'internal', message } });

/** `call(requestText) -> answerText` over the exports of an instantiated module. */
export function makeWasmCaller(exports) {
  const encoder = new TextEncoder();
  const decoder = new TextDecoder();
  return function call(text) {
    try {
      const bytes = encoder.encode(text);
      const ptr = exports.rflap_alloc(bytes.length);
      // the memory may have grown during the allocation: take the view afterwards
      new Uint8Array(exports.memory.buffer, ptr, bytes.length).set(bytes);
      const out = exports.rflap_call(ptr, bytes.length);
      exports.rflap_free(ptr, bytes.length);
      const length = new DataView(exports.memory.buffer).getUint32(out, true);
      const answer = decoder.decode(new Uint8Array(exports.memory.buffer, out + 4, length));
      exports.rflap_free_result(out);
      return answer;
    } catch (error) {
      // a panic in the module is a wasm trap (panic = abort): report it like any other error
      return internalError(`the wasm module trapped: ${error && error.message}`);
    }
  };
}

/** Compiles and instantiates the module; returns the synchronous caller. */
export async function loadWasm(bytes) {
  const { instance } = await WebAssembly.instantiate(bytes, {});
  return makeWasmCaller(instance.exports);
}
