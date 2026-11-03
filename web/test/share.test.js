import test from 'node:test';
import assert from 'node:assert/strict';
import { encodeShare, decodeShare, shareLink, documentFromHash, bytesToBase64Url, base64UrlToBytes } from '../src/share.js';
import { emptyDocument, addState, addTransition } from '../src/doc.js';

test('base64url roundtrip of every byte value, without padding or + /', () => {
  const bytes = Uint8Array.from({ length: 256 }, (_, i) => i);
  const text = bytesToBase64Url(bytes);
  assert.match(text, /^[A-Za-z0-9_-]+$/);
  assert.deepEqual(base64UrlToBytes(text), bytes);
  for (const n of [0, 1, 2, 3, 4, 5]) {
    const slice = bytes.subarray(0, n);
    assert.deepEqual(base64UrlToBytes(bytesToBase64Url(slice)), slice);
  }
});

test('a document survives the share link, including non-ASCII labels', () => {
  const doc = emptyDocument('fa');
  addState(doc, 1, 2);
  addTransition(doc, 0, 0, 'λ→ñ');
  const link = shareLink('https://example.test/app/rflap.html#old', doc);
  assert.match(link, /^https:\/\/example\.test\/app\/rflap\.html#doc=[A-Za-z0-9_-]+$/);
  const hash = link.slice(link.indexOf('#'));
  assert.deepEqual(documentFromHash(hash), doc);
  assert.deepEqual(decodeShare(encodeShare(doc)), doc);
});

test('hashes without a document, or with other parameters, are handled', () => {
  assert.equal(documentFromHash(''), null);
  assert.equal(documentFromHash('#other=1'), null);
  const value = { type: 'fa', initial: 'q0', transitions: [] };
  assert.deepEqual(documentFromHash(`#theme=dark&doc=${encodeShare(value)}`), value);
  assert.deepEqual(documentFromHash(`doc=${encodeShare(value)}`), value);
});

test('a corrupt link throws instead of returning garbage', () => {
  assert.throws(() => documentFromHash('#doc=AAAA'));
});
