// The pumping lemma texts of JFLAP carry a little HTML (<i>, <sup>, <sub>, <br>, numeric entities).
// They are shown through this tiny reader: only those tags survive, everything else is plain text,
// so engine output can never inject markup.

import { h } from './dom.js';

const NAMED = { lt: '<', gt: '>', amp: '&', quot: '"', apos: "'", nbsp: '\u00a0' };

function decodeEntities(text) {
  return text.replace(/&(#x[0-9a-f]+|#\d+|[a-z]+);/gi, (match, body) => {
    if (body[0] === '#') {
      const code = body[1] === 'x' || body[1] === 'X' ? parseInt(body.slice(2), 16) : parseInt(body.slice(1), 10);
      return Number.isFinite(code) && code > 0 && code < 0x110000 ? String.fromCodePoint(code) : match;
    }
    return NAMED[body.toLowerCase()] ?? match;
  });
}

/** `[{text, italic?, bold?, sup?, sub?} | {br: true}]` for a string with the tags above. */
export function richTokens(source) {
  if (typeof source !== 'string' || source === '') return [];
  const tokens = [];
  const open = { i: 0, b: 0, sup: 0, sub: 0 };
  const names = { i: 'italic', b: 'bold', sup: 'sup', sub: 'sub' };
  const push = (text) => {
    if (!text) return;
    const token = { text: decodeEntities(text) };
    for (const [tag, count] of Object.entries(open)) if (count > 0) token[names[tag]] = true;
    const last = tokens[tokens.length - 1];
    const sameStyle = last && !last.br && Object.keys(token).length === Object.keys(last).length && Object.keys(token).every((k) => k === 'text' || last[k] === token[k]);
    if (sameStyle) last.text += token.text;
    else tokens.push(token);
  };
  let rest = source;
  while (rest) {
    const at = rest.indexOf('<');
    if (at < 0) {
      push(rest);
      break;
    }
    push(rest.slice(0, at));
    const end = rest.indexOf('>', at);
    if (end < 0) {
      push(rest.slice(at));
      break;
    }
    const tag = /^<\s*(\/?)\s*([a-z0-9]+)/i.exec(rest.slice(at, end + 1));
    rest = rest.slice(end + 1);
    if (!tag) continue;
    const name = tag[2].toLowerCase();
    if (name === 'br') tokens.push({ br: true });
    else if (name in open) open[name] = Math.max(0, open[name] + (tag[1] ? -1 : 1));
    else if (name === 'script' || name === 'style') {
      // drop the tag only: its text is harmless once it is plain text
    }
  }
  return tokens;
}

/** DOM nodes for `richTokens`. */
export function richNodes(source) {
  return richTokens(source).map((token) => {
    if (token.br) return h('br');
    let node = token.text;
    if (token.sub) node = h('sub', {}, node);
    if (token.sup) node = h('sup', {}, node);
    if (token.italic) node = h('i', {}, node);
    if (token.bold) node = h('b', {}, node);
    return node;
  });
}

/** A span holding the rich text. */
export function richSpan(source, props = {}) {
  return h('span', props, richNodes(source));
}
