import test from 'node:test';
import assert from 'node:assert/strict';
import {
  emptyDocument, addState, removeState, moveState, setInitial, toggleFinal, renameState, setStateOutput,
  addTransition, removeTransition, setTransitionLabel, setTransitionControl, ensureBlock, removeBlock, hasBlock,
  container, machineDocument, stateByName, labelToText, labelFromText, normalizeDocument, compactOf, describe,
  bounds, clone, History, HISTORY_LIMIT,
} from '../src/doc.js';

/** A fake engine: `normalize` answers with the full form of the one-transition probe (enough to test the wiring). */
function fakeApi(log = []) {
  return async (request) => {
    log.push(request);
    assert.equal(request.op, 'normalize');
    const d = request.document;
    if (d.transitions && d.transitions.length === 1 && d.initial === 'a') {
      const text = d.transitions[0][1];
      const label = {
        fa: text === 'λ' ? '' : text,
        moore: text,
        pda: { input: 'a', pop: 'Z', push: 'AZ' },
        mealy: { input: 'a', output: '1' },
        turing: { actions: [{ read: 'a', write: 'b', direction: 'right' }] },
      }[d.type];
      const body = { finals: [], initial: 0, notes: [], states: { 0: { id: 0, name: 'a', label: null, output: null, point: { x: 0, y: 0 } } }, transitions: [{ from: 0, to: 0, control: null, label }] };
      return { document: d.type === 'turing' ? { type: 'turing', tapes: d.tapes, automaton: body, blocks: {} } : { type: d.type, ...body } };
    }
    return { document: { ...emptyDocument(d.type || 'fa'), from: 'api' } };
  };
}

test('states: add (lowest free name), rename, move, initial, final, remove with its transitions', () => {
  const doc = emptyDocument('fa');
  const a = addState(doc, 10, 20);
  const b = addState(doc, 30, 40);
  const c = addState(doc, 50, 60);
  assert.deepEqual([a, b, c], [0, 1, 2]);
  assert.deepEqual(Object.values(doc.states).map((s) => s.name), ['q0', 'q1', 'q2']);
  setInitial(doc, a);
  toggleFinal(doc, c);
  toggleFinal(doc, b);
  toggleFinal(doc, b);
  assert.deepEqual(doc.finals, [2]);
  addTransition(doc, a, b, 'x');
  addTransition(doc, b, c, 'y');
  addTransition(doc, c, c, 'z');
  moveState(doc, b, 100, 100);
  assert.deepEqual(doc.states[1].point, { x: 100, y: 100 });
  renameState(doc, b, 'mid');
  assert.equal(stateByName(doc, 'mid').id, 1);
  assert.throws(() => renameState(doc, a, 'mid'), /already used/);
  assert.throws(() => renameState(doc, a, '  '), /vacío/);
  removeState(doc, c);
  assert.deepEqual(doc.transitions.map((t) => t.label), ['x']);
  assert.deepEqual(doc.finals, []);
  removeState(doc, a);
  assert.equal(doc.initial, null);
  assert.equal(doc.transitions.length, 0);
  assert.equal(addState(doc, 0, 0), 2); // ids continue after the highest one
});

test('a new state never takes a name that is in use', () => {
  const doc = emptyDocument('fa');
  addState(doc, 0, 0);
  addState(doc, 0, 0);
  renameState(doc, 1, 'q2');
  const id = addState(doc, 0, 0);
  assert.notEqual(doc.states[id].name, 'q0');
  assert.notEqual(doc.states[id].name, 'q2');
});

test('transitions: add, relabel, set control point, remove', () => {
  const doc = emptyDocument('fa');
  addState(doc, 0, 0);
  addState(doc, 100, 0);
  const i = addTransition(doc, 0, 1, 'a');
  setTransitionLabel(doc, i, 'b');
  setTransitionControl(doc, i, { x: 50, y: -40 });
  assert.deepEqual(doc.transitions[0], { control: { x: 50, y: -40 }, from: 0, label: 'b', to: 1 });
  removeTransition(doc, i);
  assert.equal(doc.transitions.length, 0);
});

test('moving a state drags the explicit control points of its transitions', () => {
  const doc = emptyDocument('fa');
  addState(doc, 0, 0);
  addState(doc, 100, 0);
  addTransition(doc, 0, 1, 'a');
  setTransitionControl(doc, 0, { x: 50, y: 50 });
  moveState(doc, 0, 20, 0);
  assert.deepEqual(doc.transitions[0].control, { x: 60, y: 50 });
  addTransition(doc, 1, 1, 'b');
  setTransitionControl(doc, 1, { x: 100, y: -30 });
  moveState(doc, 1, 120, 10); // a loop follows by the whole move
  assert.deepEqual(doc.transitions[1].control, { x: 120, y: -20 });
});

test('moore outputs and pda documents use the same helpers', () => {
  const moore = emptyDocument('moore');
  const q = addState(moore, 0, 0);
  assert.equal(moore.states[q].output, '');
  setStateOutput(moore, q, 'x');
  assert.equal(moore.states[q].output, 'x');
  assert.equal(addState(emptyDocument('pda'), 0, 0), 0);
});

test('turing machines: states live in `automaton`, blocks are nested machines you can walk into', () => {
  const tm = emptyDocument('turing', 2);
  assert.equal(tm.tapes, 2);
  const a = addState(tm, 0, 0);
  addState(tm, 100, 0);
  assert.equal(Object.keys(tm.automaton.states).length, 2);
  assert.equal(hasBlock(tm, a), false);
  ensureBlock(tm, a);
  assert.equal(hasBlock(tm, a), true);
  assert.equal(tm.blocks[a].tapes, 2);
  assert.equal('type' in tm.blocks[a], false);
  const inner = addState(tm, 5, 5, [a]);
  assert.equal(inner, 0);
  assert.equal(container(tm, [a]).states[0].name, 'q0');
  assert.equal(machineDocument(tm, [a]).type, 'turing');
  assert.equal(machineDocument(tm, []), tm);
  ensureBlock(tm, inner, [a]); // blocks nest
  assert.equal(hasBlock(tm, inner, [a]), true);
  removeState(tm, a); // removing a state drops its block
  assert.deepEqual(tm.blocks, {});
  assert.throws(() => container(tm, [7]), /no building block/);
  ensureBlock(tm, 1);
  removeBlock(tm, 1);
  assert.equal(hasBlock(tm, 1), false);
});

test('label text uses the compact syntax of every type', () => {
  assert.equal(labelToText('fa', ''), 'λ');
  assert.equal(labelToText('fa', 'ab'), 'ab');
  assert.equal(labelToText('moore', '[0-9]'), '[0-9]');
  assert.equal(labelToText('pda', { input: 'a', pop: 'Z', push: 'AZ' }), 'a,Z;AZ');
  assert.equal(labelToText('pda', { input: '', pop: 'A', push: '' }), 'λ,A;λ');
  assert.equal(labelToText('mealy', { input: 'a', output: '1' }), 'a;1');
  assert.equal(
    labelToText('turing', { actions: [{ read: 'a', write: 'b', direction: 'right' }, { read: '□', write: 'x', direction: 'stay' }] }),
    'a;b,R|_;x,S',
  );
});

test('labelFromText asks the engine with a one-transition probe and returns the full label', async () => {
  const log = [];
  const api = fakeApi(log);
  assert.equal(await labelFromText(api, 'fa', 'λ'), '');
  assert.deepEqual(await labelFromText(api, 'pda', 'a,Z;AZ'), { input: 'a', pop: 'Z', push: 'AZ' });
  assert.deepEqual(await labelFromText(api, 'turing', 'a;b,R|a;b,R', 2), { actions: [{ read: 'a', write: 'b', direction: 'right' }] });
  assert.deepEqual(log[2].document, { type: 'turing', initial: 'a', transitions: [['a', 'a;b,R|a;b,R', 'a']], tapes: 2 });
  assert.deepEqual(log[1].document, { type: 'pda', initial: 'a', transitions: [['a', 'a,Z;AZ', 'a']] });
});

test('an engine error while parsing a label propagates (the dialog shows it)', async () => {
  const api = async () => {
    throw new Error('invalid pda label `xx`');
  };
  await assert.rejects(() => labelFromText(api, 'pda', 'xx'), /invalid pda label/);
});

test('compact -> full goes through normalize', async () => {
  const log = [];
  const full = await normalizeDocument(fakeApi(log), { type: 'fa', initial: 'q0', transitions: [] });
  assert.equal(full.from, 'api');
  assert.deepEqual(log[0], { op: 'normalize', document: { type: 'fa', initial: 'q0', transitions: [] } });
});

test('compactOf writes what a chat can read: names, texts, outputs, blocks', () => {
  const fa = emptyDocument('fa');
  const [a, b] = [addState(fa, 10, 10), addState(fa, 90, 10)];
  addState(fa, 50, 90);
  setInitial(fa, a);
  toggleFinal(fa, b);
  addTransition(fa, a, b, 'x');
  addTransition(fa, b, a, '');
  assert.deepEqual(compactOf(fa), { type: 'fa', initial: 'q0', finals: ['q1'], transitions: [['q0', 'x', 'q1'], ['q1', 'λ', 'q0']], states: ['q0', 'q1', 'q2'] });
  assert.deepEqual(compactOf(fa, { points: true }).points, { q0: [10, 10], q1: [90, 10], q2: [50, 90] });
  const moore = emptyDocument('moore');
  const m = addState(moore, 0, 0);
  setInitial(moore, m);
  setStateOutput(moore, m, 'x');
  assert.deepEqual(compactOf(moore).outputs, { q0: 'x' });
  const tm = emptyDocument('turing', 2);
  const t0 = addState(tm, 0, 0);
  setInitial(tm, t0);
  ensureBlock(tm, t0);
  addState(tm, 0, 0, [t0]);
  const compact = compactOf(tm);
  assert.equal(compact.tapes, 2);
  assert.deepEqual(Object.keys(compact.blocks), ['q0']);
  assert.equal(compact.blocks.q0.type, 'turing');
});

test('describe and bounds', () => {
  const fa = emptyDocument('fa');
  assert.equal(bounds(fa), null);
  const a = addState(fa, 10, 20);
  addState(fa, 70, 5);
  setInitial(fa, a);
  assert.deepEqual(bounds(fa), { minX: 10, minY: 5, maxX: 70, maxY: 20 });
  assert.equal(describe(fa), 'fa: 2 states, 0 transitions, initial q0, final —');
});

test('history: undo and redo, a new edit drops the redo stack, 50 levels by default', () => {
  assert.equal(HISTORY_LIMIT, 50);
  let doc = emptyDocument('fa');
  const history = new History();
  assert.equal(history.canUndo, false);
  for (let i = 0; i < 3; i++) {
    history.record(doc);
    doc = clone(doc);
    addState(doc, i, i);
  }
  assert.equal(Object.keys(doc.states).length, 3);
  doc = history.undo(doc);
  assert.equal(Object.keys(doc.states).length, 2);
  doc = history.undo(doc);
  doc = history.redo(doc);
  assert.equal(Object.keys(doc.states).length, 2);
  history.record(doc);
  assert.equal(history.canRedo, false);
  assert.equal(history.redo(doc), null);
  const small = new History();
  for (let i = 0; i < 80; i++) small.record({ i });
  assert.equal(small.past.length, 50);
  assert.equal(small.past[0].i, 30);
  assert.equal(new History().undo({}), null);
});

test('snapshots do not alias the live document', () => {
  const doc = emptyDocument('fa');
  const history = new History();
  history.record(doc);
  addState(doc, 1, 1);
  assert.equal(Object.keys(history.past[0].states).length, 0);
  const back = history.undo(doc);
  addState(back, 2, 2);
  assert.equal(Object.keys(history.future[0].states).length, 1);
});
