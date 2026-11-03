import test from 'node:test';
import assert from 'node:assert/strict';
import { pumpingView, decompositionParts, clampLengths, pumpingLanguages, lengthsFromIndices, indicesFromLengths, currentLanguage } from '../src/pumping-model.js';
import { lsystemForm, lsystemCompact, LSYSTEM_EXAMPLE } from '../src/lsystem-model.js';
import { fixture } from './fixtures.js';
import { loadApi } from './engine.js';

const pumping = fixture('pumping');

test('pumping: the game boxes keep JFLAP order and only the one the user must fill is editable', () => {
  const [start] = pumping.regularComputerFirst;
  const view = pumpingView(start, 'regular');
  assert.equal(view.boxes.length, 5);
  assert.deepEqual(view.boxes.map((b) => b.number), [1, 2, 3, 4, 5]);
  assert.deepEqual(view.boxes.map((b) => b.visible), [true, true, false, false, false]);
  assert.equal(view.boxes[0].content, '4');
  assert.equal(view.awaiting, 'w');
  assert.deepEqual(view.boxes.map((b) => b.editable), [false, true, false, false, false]);
  assert.equal(view.boxes[1].input, 'text');
  assert.equal(view.objective, 'Prevent the computer from finding a valid partition.');
  assert.equal(view.finished, false);
  assert.equal(view.result, null);
  assert.equal(view.cases, null);
  assert.equal(view.m, 4);
});

test('pumping: after the last move the result says who won and the attempt is listed', () => {
  const [, w, finish] = pumping.regularComputerFirst;
  const midway = pumpingView(w, 'regular');
  assert.equal(midway.awaiting, 'i');
  assert.equal(midway.boxes[2].content, 'X = λ;   Y = aaaa;   Z = bbbb');
  assert.equal(midway.boxes[3].editable, true);
  assert.deepEqual(midway.boxes.map((b) => b.visible), [true, true, true, true, false]);
  assert.equal(midway.reply.accepted, true);

  const end = pumpingView(finish, 'regular');
  assert.equal(end.finished, true);
  assert.equal(end.result.userWins, true);
  assert.equal(end.result.pumped, 'aaaaaaaabbbb');
  assert.equal(end.conclusion, 'not_regular');
  assert.equal(end.attempts.length, 1);
  assert.match(end.attempts[0], /Won/);
  assert.equal(end.boxes[4].visible, true);
});

test('pumping: a rejected move keeps the message in the box and the user can try again', () => {
  const view = pumpingView({ ...pumping.regularHumanFirst[0], reply: { accepted: false, message: 'Remember |w| must be >= m' } }, 'regular');
  assert.equal(view.reply.accepted, false);
  assert.equal(view.reply.message, 'Remember |w| must be >= m');
  assert.equal(view.awaiting, 'm');
  assert.equal(view.boxes[0].editable, true);
  assert.equal(view.boxes[0].input, 'number');
});

test('pumping: the user decomposes w in the context-free game, and the cases panel appears', () => {
  const [, afterM, , afterCase] = pumping.contextFreeHumanFirst;
  const view = pumpingView(afterM, 'context_free');
  assert.equal(view.awaiting, 'decomposition');
  assert.equal(view.boxes[1].content, 'aaabbbccc');
  assert.equal(view.boxes[2].input, 'decomposition');
  assert.equal(view.boxes[2].editable, true);
  assert.equal(view.parts, 4);
  assert.equal(view.cases.total, 11);
  assert.equal(view.cases.listEnabled, true);
  const withCase = pumpingView(afterCase, 'context_free');
  assert.deepEqual(withCase.cases.rows, ['v is a non-empty string and y is an empty string']);
  assert.equal(withCase.cases.selected, 0);
  assert.equal(withCase.cases.addReplaceEnabled, true);
  assert.equal(withCase.result.userWins, false);
  assert.match(withCase.result.takeaway, /I WIN/);
  assert.equal(pumpingView(pumping.regularHumanFirst[1], 'regular').parts, 2);
});

test('pumping: titles and explanations are plain text safe to show (markup only as tokens)', () => {
  const view = pumpingView(pumping.regularHumanFirst[0], 'regular');
  assert.equal(view.title, 'L = {a^n b^n : n >= 0} Regular Pumping Lemma');
  assert.equal(view.languageHtml, 'L = {<i>a<sup>n</sup>b<sup>n</sup></i> : <i>n</i> &#8805; 0}');
});

test('pumping: decomposition lengths split w into x y z (or u v x y z) and are clamped to w', () => {
  assert.deepEqual(decompositionParts('aaaabbbb', [0, 4]), ['', 'aaaa', 'bbbb']);
  assert.deepEqual(decompositionParts('aaaabbbb', [2, 3]), ['aa', 'aab', 'bbb']);
  assert.deepEqual(decompositionParts('aaabbbccc', [0, 1, 1, 0]), ['', 'a', 'a', '', 'abbbccc']);
  assert.deepEqual(decompositionParts('ab', [5, 5]), ['ab', '', ''], 'lengths past the end are cut');
  assert.deepEqual(decompositionParts('', [0, 0]), ['', '', '']);
  assert.deepEqual(clampLengths('aaaabbbb', [3, 9]), [3, 5]);
  assert.deepEqual(clampLengths('aaaabbbb', [9, 1]), [8, 0]);
  assert.deepEqual(clampLengths('abc', [-2, 1]), [0, 1]);
  assert.deepEqual(clampLengths('abcdef', [1, 2, 3, 4]), [1, 2, 3, 0]);
  assert.deepEqual(clampLengths('abc', []), [0, 0]);
  assert.deepEqual(clampLengths('abc', [], 4), [0, 0, 0, 0]);
});

test('pumping: index inputs (where the pieces start) become lengths', () => {
  assert.deepEqual(lengthsFromIndices([1, 3]), [1, 2]);
  assert.deepEqual(lengthsFromIndices([0, 1, 2, 3]), [0, 1, 1, 1]);
  assert.deepEqual(lengthsFromIndices([4, 2]), [4, 0], 'indices out of order give an empty piece, never a negative length');
});

test('pumping: the catalogue is filtered by lemma and keeps the engine index', () => {
  const regular = pumpingLanguages(pumping.languages, 'regular');
  assert.deepEqual(regular.map((l) => l.index), [0, 1]);
  assert.equal(regular[0].title, 'a^n b^n : n >= 0');
  assert.equal(pumpingLanguages(pumping.languages, 'context_free').every((l) => l.kind === 'context_free'), true);
  assert.equal(pumping.languages.total, 26);
});

test('pumping (engine): a whole game played through the model reaches a verdict', async () => {
  const api = await loadApi();
  let game = await api({ op: 'pumping_new', kind: 'context_free', language: 0, mode: 'computer_first', seed: 3 });
  let view = pumpingView(game, 'context_free');
  assert.equal(view.awaiting, 'w');
  game = await api({ op: 'pumping_move', game: game.document, move: { move: 'choose_w', w: 'a'.repeat(view.m) + 'b'.repeat(view.m) + 'c'.repeat(view.m) } });
  view = pumpingView(game, 'context_free');
  assert.equal(view.awaiting, 'i');
  game = await api({ op: 'pumping_move', game: game.document, move: { move: 'choose_i', i: 0 } });
  view = pumpingView(game, 'context_free');
  assert.equal(view.finished, true);
  assert.equal(typeof view.result.userWins, 'boolean');
});

// ---------------------------------------------------------------- L-systems

test('l-system form: the full document becomes axiom, rule rows and parameter rows', () => {
  const form = lsystemForm(fixture('lsystem').normalized);
  assert.deepEqual(form, {
    axiom: 'g',
    rules: [{ lhs: 'g', rhs: 'g + g - g - g + g' }],
    parameters: [{ name: 'angle', value: '90' }, { name: 'distance', value: '4' }],
  });
});

test('l-system form: several replacements of one symbol are several rows', () => {
  const form = lsystemForm({ type: 'lsystem', axiom: ['X'], rules: { X: [['g', 'X'], ['f', 'X']] }, parameters: [] });
  assert.deepEqual(form.rules, [{ lhs: 'X', rhs: 'g X' }, { lhs: 'X', rhs: 'f X' }]);
});

test('l-system form: rows become the compact document; blank rows vanish; bad rows are explained', () => {
  const compact = lsystemCompact({ axiom: ' g ', rules: [{ lhs: 'g', rhs: 'g + g' }, { lhs: '', rhs: '' }], parameters: [{ name: 'angle', value: '90' }, { name: ' ', value: '' }] });
  assert.deepEqual(compact, { type: 'lsystem', axiom: 'g', rules: ['g -> g + g'], parameters: { angle: '90' } });
  assert.throws(() => lsystemCompact({ axiom: ' ', rules: [], parameters: [] }), /axiom/i);
  assert.throws(() => lsystemCompact({ axiom: 'g', rules: [{ lhs: '', rhs: 'g' }], parameters: [] }), /left side/i);
  assert.throws(() => lsystemCompact({ axiom: 'g', rules: [{ lhs: 'g h', rhs: 'g' }], parameters: [] }), /one symbol/i);
  assert.throws(() => lsystemCompact({ axiom: 'g', rules: [], parameters: [{ name: '', value: '90' }] }), /parameter/i);
  assert.deepEqual(lsystemCompact({ axiom: 'g', rules: [{ lhs: 'g', rhs: '' }], parameters: [] }).rules, ['g -> ']);
});

test('l-system (engine): the example is accepted, normalizes back to the same rows and draws', async () => {
  const api = await loadApi();
  const compact = lsystemCompact(LSYSTEM_EXAMPLE);
  const { document } = await api({ op: 'normalize', document: compact });
  assert.deepEqual(lsystemForm(document), LSYSTEM_EXAMPLE);
  const drawn = await api({ op: 'lsystem_render', document: compact, depth: 2 });
  assert.match(drawn.svg, /<line /);
  assert.equal(drawn.depth, 2);
});

test('pumping: cut points are the running sum of the lengths', () => {
  assert.deepEqual(indicesFromLengths([1, 2]), [1, 3]);
  assert.deepEqual(indicesFromLengths([0, 1, 1, 1]), [0, 1, 2, 3]);
  assert.deepEqual(lengthsFromIndices(indicesFromLengths([2, 0, 3])), [2, 0, 3]);
});

test('pumping: the game on the table is found in the catalogue by its title', () => {
  const view = pumpingView(pumping.regularHumanFirst[0], 'regular');
  const languages = pumpingLanguages(pumping.languages, 'regular');
  assert.equal(currentLanguage(view.title, languages).index, 0);
  assert.equal(currentLanguage('nothing like it', languages), null);
  assert.equal(currentLanguage('w a^n b^nn', [{ title: 'a^n b^n' }, { title: 'a^n b^nn', index: 7 }]).index, 7, 'the longest title wins');
});
