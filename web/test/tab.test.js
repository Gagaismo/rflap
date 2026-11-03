import test from 'node:test';
import assert from 'node:assert/strict';
import { Tab } from '../src/tab.js';
import { emptyDocument, addState, addTransition, clone, ensureBlock, stateList } from '../src/doc.js';
import { splitStructured } from '../src/app.js';

const app = { api: async () => ({}), host: { mode: 'standalone' }, t: (k) => k, dark: false, notify() {}, tabs: () => [], openTab() {} };
const newTab = (doc = emptyDocument('fa')) => new Tab(app, { doc, title: 'x' });

test('commit records history and notifies; undo/redo restore documents and notify', async () => {
  const tab = newTab();
  const events = [];
  tab.on('doc', (e) => events.push(e.cause));
  const next = clone(tab.doc);
  addState(next, 1, 1);
  await tab.commit(next);
  assert.equal(stateList(tab.doc).length, 1);
  assert.equal(tab.dirty, true);
  assert.equal(tab.undo(), true);
  assert.equal(stateList(tab.doc).length, 0);
  assert.equal(tab.redo(), true);
  assert.equal(stateList(tab.doc).length, 1);
  assert.equal(tab.undo() && tab.undo(), false);
  assert.deepEqual(events, ['edit', 'undo', 'redo', 'undo']);
});

test('edit applies a change to a clone; an error leaves the document untouched and is reported', async () => {
  const notified = [];
  const tab = new Tab({ ...app, notify: (m, k) => notified.push([m, k]) }, { doc: emptyDocument('fa'), title: 'x' });
  const before = tab.doc;
  assert.equal(await tab.edit((d) => addState(d, 5, 5)), true);
  assert.notEqual(tab.doc, before);
  assert.equal(before.states[0], undefined);
  const current = tab.doc;
  assert.equal(await tab.edit(() => { throw new Error('nope'); }), false);
  assert.equal(tab.doc, current);
  assert.deepEqual(notified, [['nope', 'error']]);
  assert.equal(tab.history.past.length, 1);
});

test('replace resets history, path and overlays', async () => {
  const tab = newTab();
  await tab.edit((d) => addState(d, 0, 0));
  tab.setOverlay('sim', { states: [0] });
  tab.replace(emptyDocument('pda'));
  assert.equal(tab.history.canUndo, false);
  assert.equal(tab.type, 'pda');
  assert.equal(tab.overlay().states.size, 0);
});

test('overlays from several sources are unioned and a source can be cleared', () => {
  const tab = newTab();
  tab.setOverlay('sim', { states: [1, 2], transitions: [0] });
  tab.setOverlay('steps', { states: [2, 3] });
  assert.deepEqual([...tab.overlay().states].sort(), [1, 2, 3]);
  assert.deepEqual([...tab.overlay().transitions], [0]);
  tab.setOverlay('sim', null);
  assert.deepEqual([...tab.overlay().states].sort(), [2, 3]);
});

test('walking into a building block and back, and undo that removes the block drops the path', async () => {
  const tab = newTab(emptyDocument('turing', 1));
  await tab.edit((d) => {
    const id = addState(d, 0, 0);
    ensureBlock(d, id);
    addTransition(d, id, id, { actions: [] });
  });
  const paths = [];
  tab.on('path', () => paths.push([...tab.path]));
  tab.enterBlock(0);
  assert.deepEqual(tab.path, [0]);
  tab.exitBlock();
  assert.deepEqual(tab.path, []);
  tab.enterBlock(0);
  tab.undo(); // the block no longer exists
  assert.deepEqual(tab.path, []);
  assert.deepEqual(paths.at(-1), []);
});

test('flags notify so the canvas redraws', () => {
  const tab = newTab();
  const seen = [];
  tab.on('flags', (n) => seen.push(n));
  tab.on('doc', (e) => seen.push(e.cause));
  tab.setFlag('lambda', true);
  assert.equal(tab.flags.lambda, true);
  assert.deepEqual(seen, ['lambda', 'flags']);
});

test('splitStructured finds the document and keeps the rest as extras', () => {
  const doc = { type: 'fa', initial: 'q0', transitions: [] };
  assert.deepEqual(splitStructured({ document: doc, svg: '<svg/>', analysis: { kind: 'dfa' }, note: null }), { document: doc, extras: { svg: '<svg/>', analysis: { kind: 'dfa' } } });
  assert.deepEqual(splitStructured({ result: { document: doc, steps: [1] }, op: 'convert' }).document, doc);
  assert.deepEqual(splitStructured({ result: { document: doc, steps: [1] }, op: 'convert' }).extras, { steps: [1], op: 'convert' });
  assert.deepEqual(splitStructured(doc), { document: doc, extras: {} });
  assert.deepEqual(splitStructured({ svg: '<svg/>' }), { document: null, extras: { svg: '<svg/>' } });
  assert.deepEqual(splitStructured(null), { document: null, extras: {} });
});
