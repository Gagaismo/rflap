// A tiny, safe Markdown reader for the text an AI server sends (an `explanation` with tables, lists, `code`).
// `parseMarkdown` is pure (blocks of plain data); `markdownNodes` builds DOM with h(), so model text is only
// ever set as text and can never inject markup.

import { h } from './dom.js';

const ROW = /^\s*\|.*\|\s*$/;
const SEPARATOR = /^\s*\|?\s*:?-{2,}:?\s*(\|\s*:?-{2,}:?\s*)*\|?\s*$/;

/** True when the text holds a pipe table (a row followed by a `---|---` separator row). */
export function hasTable(text) {
  if (typeof text !== 'string') return false;
  const lines = text.split(/\r?\n/);
  return lines.some((line, i) => ROW.test(line) && i + 1 < lines.length && SEPARATOR.test(lines[i + 1]) && lines[i + 1].includes('|'));
}

const cells = (line) => {
  let body = line.trim();
  if (body.startsWith('|')) body = body.slice(1);
  if (body.endsWith('|') && !body.endsWith('\x5c|')) body = body.slice(0, -1);
  return body.split(/(?<!\x5c)\|/).map((cell) => cell.replace(/\x5c\|/g, '|').trim());
};

/** Inline tokens: `[{text}, {code}, {bold}]` for `code`, **bold** and plain text. */
export function parseInline(source) {
  const tokens = [];
  const pattern = /`([^`]+)`|\*\*([^*]+)\*\*/g;
  let last = 0;
  for (const match of String(source).matchAll(pattern)) {
    if (match.index > last) tokens.push({ text: source.slice(last, match.index) });
    tokens.push(match[1] !== undefined ? { code: match[1] } : { bold: match[2] });
    last = match.index + match[0].length;
  }
  if (last < source.length) tokens.push({ text: source.slice(last) });
  return tokens;
}

/** Blocks: `{type: 'heading', level, text} | {type: 'paragraph', text} | {type: 'list', ordered, items} | {type: 'table', head, rows}`. */
export function parseMarkdown(source) {
  const lines = typeof source === 'string' ? source.split(/\r?\n/) : [];
  const blocks = [];
  let paragraph = [];
  const flush = () => {
    if (paragraph.length) blocks.push({ type: 'paragraph', text: paragraph.join(' ') });
    paragraph = [];
  };
  for (let i = 0; i < lines.length; i += 1) {
    const line = lines[i];
    let m;
    if (!line.trim()) flush();
    else if ((m = /^\s{0,3}(#{1,6})\s+(.*?)\s*#*\s*$/.exec(line))) {
      flush();
      blocks.push({ type: 'heading', level: m[1].length, text: m[2] });
    } else if (ROW.test(line) && i + 1 < lines.length && SEPARATOR.test(lines[i + 1])) {
      flush();
      const head = cells(line);
      const rows = [];
      i += 2;
      while (i < lines.length && ROW.test(lines[i])) rows.push(cells(lines[i++]));
      i -= 1;
      blocks.push({ type: 'table', head, rows });
    } else if ((m = /^\s*([-*+]|\d+[.)])\s+(.*)$/.exec(line))) {
      flush();
      const ordered = /\d/.test(m[1]);
      const last = blocks[blocks.length - 1];
      if (last && last.type === 'list' && last.ordered === ordered) last.items.push(m[2]);
      else blocks.push({ type: 'list', ordered, items: [m[2]] });
    } else paragraph.push(line.trim());
  }
  flush();
  return blocks;
}

const inlineNodes = (text) => parseInline(text).map((t) => (t.code !== undefined ? h('code', {}, t.code) : t.bold !== undefined ? h('strong', {}, t.bold) : t.text));

/** DOM nodes for a Markdown string. */
export function markdownNodes(source) {
  return parseMarkdown(source).map((block) => {
    if (block.type === 'heading') return h(`h${Math.min(6, block.level + 2)}`, {}, inlineNodes(block.text));
    if (block.type === 'list') return h(block.ordered ? 'ol' : 'ul', {}, block.items.map((item) => h('li', {}, inlineNodes(item))));
    if (block.type === 'table') {
      return h(
        'table',
        { class: 'configs' },
        h('thead', {}, h('tr', {}, block.head.map((c) => h('th', { scope: 'col' }, inlineNodes(c))))),
        h('tbody', {}, block.rows.map((row) => h('tr', {}, block.head.map((_, i) => h('td', {}, inlineNodes(row[i] ?? '')))))),
      );
    }
    return h('p', {}, inlineNodes(block.text));
  });
}
