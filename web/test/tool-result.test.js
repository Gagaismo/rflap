import test from 'node:test';
import assert from 'node:assert/strict';
import { Tab } from '../src/tab.js';
import { emptyDocument } from '../src/doc.js';
import { contentExtras, resultNotation, resultTitle } from '../src/tool-result.js';
import { hasTable, parseInline, parseMarkdown } from '../src/markdown.js';
import { splitStructured } from '../src/app.js';

const app = { api: async () => ({}), host: { mode: 'standalone' }, t: (k) => k, notation: 'catedra', notify() {}, tabs: () => [], openTab() {} };

test('S2/S3: the title of a result names the tab; no title keeps the generic one', () => {
  assert.equal(resultTitle({ title: '  2.1 a) a^n b^n ', document: {} }), '2.1 a) a^n b^n');
  assert.equal(resultTitle({ document: {} }), null);
  assert.equal(resultTitle({ title: '   ' }), null);
  assert.equal(resultTitle({ title: 7 }), null);
  assert.equal(resultTitle(null), null);
});

test('S5: tool, title and notation are not content for the Assistant panel', () => {
  const { extras } = splitStructured({ document: { type: 'fa' }, tool: 'rflap_show', title: 'T', notation: 'catedra', explanation: 'x' });
  assert.deepEqual(contentExtras(extras), { explanation: 'x' });
  assert.deepEqual(contentExtras({ tool: 'rflap_show' }), {});
});

test('S7: a result asks for a known notation only', () => {
  assert.equal(resultNotation({ notation: 'jflap' }), 'jflap');
  assert.equal(resultNotation({ notation: 'catedra' }), 'catedra');
  assert.equal(resultNotation({ notation: 'x' }), null);
  assert.equal(resultNotation({}), null);
});

test('S7: a tab uses the notation of its result over the app choice, and goes back when cleared', () => {
  const tab = new Tab(app, { doc: emptyDocument('turing'), title: 'x' });
  const seen = [];
  tab.on('notation', (n) => seen.push(n));
  assert.equal(tab.notation, 'catedra');
  tab.setNotationOverride('jflap');
  assert.equal(tab.notation, 'jflap');
  tab.setNotationOverride(null);
  assert.equal(tab.notation, 'catedra');
  assert.deepEqual(seen, ['jflap', 'catedra']);
});

test('S3: the caption of a tab is set, announced and cleared', () => {
  const tab = new Tab(app, { doc: emptyDocument('fa'), title: 'x' });
  const seen = [];
  tab.on('caption', (c) => seen.push(c));
  assert.equal(tab.caption, null);
  tab.setCaption('M1');
  tab.setCaption(null);
  assert.deepEqual(seen, ['M1', null]);
});

const TABLE = '| Estado | a | b |\n|---|---|---|\n| q0 | (q1,X,D) | Ø |\n| q1 | Ø | (q0,Y,I) |';

test('S5: markdown headings, paragraphs, bullets and a pipe table become blocks', () => {
  const blocks = parseMarkdown(`## Tabla\nUna línea\ny otra.\n\n- uno\n- dos con \`code\`\n\n${TABLE}`);
  assert.deepEqual(blocks.map((b) => b.type), ['heading', 'paragraph', 'list', 'table']);
  assert.equal(blocks[1].text, 'Una línea y otra.');
  assert.deepEqual(blocks[2].items, ['uno', 'dos con `code`']);
  assert.deepEqual(blocks[3].head, ['Estado', 'a', 'b']);
  assert.deepEqual(blocks[3].rows, [['q0', '(q1,X,D)', 'Ø'], ['q1', 'Ø', '(q0,Y,I)']]);
});

test('S5: symbols stay as text and markup in model text is never interpreted', () => {
  assert.deepEqual(parseInline('Δ y <b>x</b> `Ø`'), [{ text: 'Δ y <b>x</b> ' }, { code: 'Ø' }]);
  const [p] = parseMarkdown('<script>alert(1)</script>');
  assert.equal(p.text, '<script>alert(1)</script>');
});

test('S5: only a string with a table row plus separator counts as a table', () => {
  assert.equal(hasTable(TABLE), true);
  assert.equal(hasTable('a | b'), false);
  assert.equal(hasTable(undefined), false);
});
