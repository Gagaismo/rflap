import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import vm from 'node:vm';
import { fileURLToPath } from 'node:url';

const root = path.join(path.dirname(fileURLToPath(import.meta.url)), '..');
const html = fs.readFileSync(path.join(root, 'dist/rflap.html'), 'utf8');
const ALLOWED = new Set(['http://www.w3.org/2000/svg', 'http://www.w3.org/1999/xlink']);

test('the built page loads nothing from outside (no external URLs, no CDN, no fonts)', () => {
  const urls = [...html.matchAll(/\b(?:https?|wss?|ftp):\/\/[^\s"'<>)`]+/g)].map((m) => m[0]);
  assert.deepEqual(urls.filter((u) => !ALLOWED.has(u)), []);
  assert.doesNotMatch(html, /<link\b/i);
  assert.doesNotMatch(html, /<script[^>]*\ssrc=/i);
  const style = /<style>([\s\S]*?)<\/style>/.exec(html)[1];
  assert.doesNotMatch(style, /@import|url\(\s*['"]?(?!data:)/i);
  assert.doesNotMatch(html, /<img[^>]*\ssrc=["']?(?!data:)/i);
});

test('it is ONE self-contained file: inline style, inline script, wasm as base64', () => {
  assert.equal((html.match(/<style>/g) || []).length, 1);
  assert.equal((html.match(/<script\b/g) || []).length, 2);
  assert.match(html, /<script id="rflap-wasm" type="application\/octet-stream">[A-Za-z0-9+/=]{1000,}<\/script>/);
  assert.match(html, /<div id="app">/);
});

test('the bundled script parses and contains every module of web/src, once', () => {
  const script = /<script>\n\(\(\) => \{([\s\S]*?)\n\}\)\(\);\n<\/script>/.exec(html);
  assert.ok(script, 'bundle script not found');
  new vm.Script(`(function(){${script[1]}})`); // a duplicate top-level name or a leftover import/export fails here
  const modules = fs.readdirSync(path.join(root, 'src')).filter((f) => f.endsWith('.js'));
  for (const name of modules) assert.equal(script[1].split(`// ---- ${name}\n`).length - 1, 1, `${name} must be bundled once (is dist/rflap.html stale? run: cargo run -p xtask -- build-web)`);
  assert.doesNotMatch(script[1], /^\s*(import|export)\s/m);
});

test('dist/rflap.html is up to date with the CSS and the sources', () => {
  const css = fs.readFileSync(path.join(root, 'src/style.css'), 'utf8');
  assert.ok(html.includes(css), 'style.css changed: rebuild with cargo run -p xtask -- build-web');
  const i18n = fs.readFileSync(path.join(root, 'src/i18n.js'), 'utf8');
  const sample = /'menu\.file': '([^']+)'/.exec(i18n)[1];
  assert.ok(html.includes(`'menu.file': '${sample}'`));
});

test('its size stays reasonable for an MCP resource', () => {
  assert.ok(html.length < 4 * 1024 * 1024, `${html.length} bytes`);
});
