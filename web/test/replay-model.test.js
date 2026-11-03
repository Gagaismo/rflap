import test from 'node:test';
import assert from 'node:assert/strict';
import { subsetFrames, partitionFrames, prunedTree, gtgFrames, gtgDocument, regexNfaFrames, nfaGraphDocument, conversionKind, treeNodeLabel } from '../src/replay-model.js';
import { fixture } from './fixtures.js';
import { loadApi } from './engine.js';

const conv = fixture('conversions');

const edgeKey = (e) => `${e.from}-${e.label}->${e.to}`;

// ---------------------------------------------------------------- what kind of conversion a list of steps is

test('conversionKind recognises the step lists of the conversions', () => {
  assert.equal(conversionKind(conv.dfa.steps), 'subset');
  assert.equal(conversionKind(conv.minimal.steps), 'minimize');
  assert.equal(conversionKind(conv.toRegex.steps), 'gtg');
  assert.equal(conversionKind(conv.regexNfa.steps), 'regex-nfa');
  assert.equal(conversionKind([{ kind: 'something_else' }]), null);
  assert.equal(conversionKind([]), null);
  assert.equal(conversionKind(null), null);
});

// ---------------------------------------------------------------- NFA -> DFA

test('subset construction: every new DFA state shows the NFA states it stands for, transitions accumulate', () => {
  const frames = subsetFrames(conv.dfa.steps);
  assert.equal(frames.length, conv.dfa.steps.length);
  assert.deepEqual(frames[0].states, [{ state: 0, nfa: [0] }]);
  assert.deepEqual(frames[0].added, { state: 0, nfa: [0] });
  assert.deepEqual(frames[1].states.map((s) => s.nfa), [[0], [0, 1]]);
  assert.deepEqual(frames[2].transitions, [{ from: 0, symbol: 'a', to: 1 }]);
  assert.equal(frames[2].added, null);
  const last = frames.at(-1);
  assert.deepEqual(last.states.map((s) => s.nfa), conv.dfa.dfa_states.map((s) => s.nfa_states));
  assert.equal(last.transitions.length, conv.dfa.document.transitions.length);
  assert.deepEqual(last.focus, { states: [3, 0], nfa: [] }, 'the last step is the transition q3 -b-> q0');
  assert.deepEqual(frames[1].focus, { states: [1], nfa: [0, 1] });
  assert.equal(frames[0].states === frames[1].states, false, 'frames are snapshots');
});

// ---------------------------------------------------------------- minimization

test('minimization: the partition is refined split by split and merged at the end', () => {
  const frames = partitionFrames(conv.minimal.steps);
  assert.equal(frames.length, conv.minimal.steps.length);
  assert.deepEqual(frames[0].groups, [[0, 1, 2], [3]]);
  assert.deepEqual(frames[1].groups, [[0, 1], [2], [3]]);
  assert.deepEqual(frames[1].changed, [[0, 1], [2]]);
  assert.deepEqual(frames[2].groups, [[0], [1], [2], [3]]);
  assert.deepEqual(frames.at(-1).groups, [[0], [1], [2], [3]]);
  assert.equal(frames[1].symbol, 'b');
  assert.equal(frames.at(-1).merge, 3);
});

test('minimization: the distinguishability tree grows split by split', () => {
  const frames = partitionFrames(conv.minimal.steps);
  const shown = (frame) => {
    const out = [];
    const walk = (n) => {
      out.push(n.states.join(','));
      n.children.forEach(walk);
    };
    walk(prunedTree(conv.minimal.tree, frame.seen));
    return out;
  };
  assert.deepEqual(shown(frames[0]), ['0,1,2,3', '0,1,2', '3']);
  assert.deepEqual(shown(frames[1]), ['0,1,2,3', '0,1,2', '0,1', '2', '3']);
  assert.deepEqual(shown(frames[2]).length, 7);
  const first = prunedTree(conv.minimal.tree, frames[0].seen);
  assert.equal(first.children[0].symbol, null, 'a group that has not been split shows no symbol yet');
  assert.equal(prunedTree(conv.minimal.tree, frames[1].seen).children[0].symbol, 'b');
  assert.equal(treeNodeLabel({ states: [0, 1] }, (id) => `q${id}`), '{q0, q1}');
});

// ---------------------------------------------------------------- FA -> RE (generalized transition graph)

test('FA to RE: the graph is rebuilt step by step with the labels the engine reports', () => {
  const frames = gtgFrames(conv.gtgSource, conv.toRegex.steps);
  assert.equal(frames.length, conv.toRegex.steps.length);
  assert.deepEqual(frames[0].states, [0, 1, 2]);
  assert.deepEqual(frames[0].finals, [2]);
  assert.deepEqual(frames[0].edges.map(edgeKey).sort(), ['0-a->1', '0-λ->2', '1-b->0', '1-c->1', '1-d->1', '1-λ->2'].sort());
  assert.deepEqual(frames[1].edges.filter((e) => e.from === 1 && e.to === 1).map((e) => e.label), ['c+d']);
  assert.equal(frames[2].edges.length, frames[1].edges.length, 'ø edges exist but are not drawn');
  assert.equal(frames[2].emptyPairs, 4);
  const elim = frames[3];
  assert.deepEqual(elim.states, [0, 2]);
  assert.equal(elim.eliminated, 1);
  assert.deepEqual(elim.edges.map(edgeKey), ['0-a(c+d)*b->0', '0-λ+a(c+d)*->2']);
  assert.equal(frames[4].expression, '(a(c+d)*b)*(λ+a(c+d)*)');
  assert.deepEqual(frames[4].states, [0, 2]);
});

test('FA to RE: without extra final states the first step is already the collapse / ø step', () => {
  const frames = gtgFrames(conv.nfaSource, conv.toRegexSimple.steps);
  assert.equal(frames.length, conv.toRegexSimple.steps.length);
  assert.deepEqual(frames[0].states, [0, 1, 2, 3]);
  assert.deepEqual(frames[0].edges.filter((e) => e.from === 0 && e.to === 0).map((e) => e.label), ['a+b']);
  assert.deepEqual(frames.at(-2).states, [0, 3]);
  assert.equal(frames.at(-1).expression, conv.toRegexSimple.expression);
});

test('FA to RE: the frame becomes a document the engine can draw (positions of the source, one for the new final state)', async () => {
  const frames = gtgFrames(conv.gtgSource, conv.toRegex.steps);
  const doc = gtgDocument(conv.gtgSource, frames[0]);
  assert.equal(doc.type, 'fa');
  assert.deepEqual(Object.keys(doc.states), ['0', '1', '2']);
  assert.deepEqual(doc.finals, [2]);
  assert.equal(doc.initial, 0);
  assert.equal(typeof doc.states[2].point.x, 'number');
  assert.equal(doc.transitions.length, 6);
  assert.equal(doc.transitions.find((t) => t.from === 0 && t.to === 2).label, '', 'λ is the empty label');
  const later = gtgDocument(conv.gtgSource, frames[3]);
  assert.deepEqual(Object.keys(later.states), ['0', '2']);
  assert.equal(later.transitions.find((t) => t.from === 0 && t.to === 0).label, 'a(c+d)*b');
  const api = await loadApi();
  for (const frame of frames) {
    const rendered = await api({ op: 'render', document: gtgDocument(conv.gtgSource, frame), format: 'svg' });
    assert.match(rendered.content, /^<svg/);
  }
});

// ---------------------------------------------------------------- RE -> NFA (de-expressionify)

test('RE to NFA: the generalized graph after the last step is the NFA the engine returned', () => {
  const frames = regexNfaFrames(conv.regexNfa.steps);
  assert.equal(frames.length, 4);
  assert.deepEqual(frames[0].edges.map(edgeKey).slice(0, 2), ['2-(a+b)*->3', '4-a->5']);
  assert.equal(frames[0].action, 'decat');
  const finalDoc = conv.regexNfa.document;
  const wanted = finalDoc.transitions.map((t) => `${t.from}-${t.label === '' ? 'λ' : t.label}->${t.to}`).sort();
  assert.deepEqual(frames.at(-1).edges.map(edgeKey).sort(), wanted);
  assert.deepEqual(frames.at(-1).states, Object.keys(finalDoc.states).map(Number));
  assert.deepEqual(frames[1].replaced, { from: 2, to: 3, label: '(a+b)*' });
  assert.deepEqual(frames[1].addedStates, [10, 11]);
});

test('RE to NFA: every frame draws (the engine renders the graph at each step)', async () => {
  const api = await loadApi();
  const frames = regexNfaFrames(conv.regexNfa.steps);
  for (const frame of frames) {
    const doc = nfaGraphDocument(conv.regexNfa.document, frame);
    assert.equal(doc.initial, 0);
    assert.deepEqual(doc.finals, [1]);
    const rendered = await api({ op: 'render', document: doc, format: 'svg', highlight_states: frame.addedStates });
    assert.match(rendered.content, /^<svg/);
  }
  const first = nfaGraphDocument(conv.regexNfa.document, regexNfaFrames(conv.regexNfa.steps)[0]);
  assert.equal(first.transitions.length, 9);
});

test('step replay is stable on an empty list of steps', () => {
  assert.deepEqual(subsetFrames([]), []);
  assert.deepEqual(partitionFrames([]), []);
  assert.deepEqual(gtgFrames(conv.gtgSource, []), []);
  assert.deepEqual(regexNfaFrames([]), []);
});
