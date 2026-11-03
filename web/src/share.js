// Share links: `<page>#doc=<base64url of the UTF-8 JSON of the document>` (no padding).

export function bytesToBase64Url(bytes) {
  let binary = '';
  for (let i = 0; i < bytes.length; i += 0x8000) {
    binary += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  }
  return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

export function base64UrlToBytes(text) {
  const normal = text.replace(/-/g, '+').replace(/_/g, '/');
  const binary = atob(normal + '='.repeat((4 - (normal.length % 4)) % 4));
  return Uint8Array.from(binary, (c) => c.charCodeAt(0));
}

export function encodeShare(value) {
  return bytesToBase64Url(new TextEncoder().encode(JSON.stringify(value)));
}

export function decodeShare(text) {
  return JSON.parse(new TextDecoder().decode(base64UrlToBytes(text.trim())));
}

/** The link to open `document` in the page at `base` (any existing hash of `base` is dropped). */
export function shareLink(base, document) {
  return `${base.split('#')[0]}#doc=${encodeShare(document)}`;
}

/** The document in a location hash (`#doc=...`), or null when there is none. Throws if it is corrupt. */
export function documentFromHash(hash) {
  const match = /^#?(?:.*&)?doc=([A-Za-z0-9_-]+)/.exec(hash || '');
  return match ? decodeShare(match[1]) : null;
}
