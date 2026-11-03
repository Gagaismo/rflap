// Loads the wasm engine embedded in dist/rflap.html for the tests (and for recording fixtures).
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { decodeBase64, loadWasm } from '../src/wasm.js';
import { createApi, localTransport } from '../src/api.js';

const dist = path.join(path.dirname(fileURLToPath(import.meta.url)), '../dist/rflap.html');

export async function loadApi() {
  const html = fs.readFileSync(dist, 'utf8');
  const base64 = /<script id="rflap-wasm" type="application\/octet-stream">([^<]+)<\/script>/.exec(html)[1];
  const call = await loadWasm(decodeBase64(base64));
  return createApi(() => localTransport(call));
}
