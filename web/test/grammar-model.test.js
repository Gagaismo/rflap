import test from 'node:test';
import assert from 'node:assert/strict';
import {
  parseProductionText, serializeProductions, addGrammarRow, removeGrammarRow, moveGrammarRow, setGrammarRow, productionText,
  grammarPhases, grammarFrames, replayedProductions, describeGrammarStep, tablesModel, describeParseAction, ll1Trace, slrTrace, cykModel,
  userMove, withoutLastMove, userChoices, derivationSteps, parseInputLines, startVariable,
} from '../src/grammar-model.js';
import { fixture } from './fixtures.js';
import { loadApi } from './engine.js';
import { setLanguage } from '../src/i18n.js';

setLanguage('en'); // these tests assert the English messages; the app default is Spanish

const parse = fixture('parse');
const transforms = fixture('grammar-transforms');
const key = (p) => `${p.lhs}→${p.rhs}`;
const asSet = (list) => [...new Set(list.map(key))].sort();

// ---------------------------------------------------------------- the production table

test('production text: alternatives, λ / ε, arrows, blank lines and comments', () => {
  const { rows, errors } = parseProductionText('S -> aSb | λ\n\n# a comment\nA → a|ε\nB ->\n  C   ->  c d ');
  assert.deepEqual(errors, []);
  assert.deepEqual(rows, [
    { lhs: 'S', rhs: 'aSb' }, { lhs: 'S', rhs: '' }, { lhs: 'A', rhs: 'a' }, { lhs: 'A', rhs: '' }, { lhs: 'B', rhs: '' }, { lhs: 'C', rhs: 'cd' },
  ]);
});

test('production text: a line without an arrow or without a left side is an error with its line number', () => {
  const { rows, errors } = parseProductionText('S -> a\nS aSb\n-> b\nT -> t');
  assert.deepEqual(rows, [{ lhs: 'S', rhs: 'a' }, { lhs: 'T', rhs: 't' }]);
  assert.deepEqual(errors.map((e) => e.line), [2, 3]);
  assert.match(errors[0].message, /->/);
  assert.match(errors[1].message, /left side/);
  assert.deepEqual(parseProductionText(''), { rows: [], errors: [] });
});

test('production text: serialize groups consecutive productions of one variable and round-trips', () => {
  const rows = [{ lhs: 'S', rhs: 'aSb' }, { lhs: 'S', rhs: '' }, { lhs: 'A', rhs: 'a' }, { lhs: 'S', rhs: 'c' }];
  const text = serializeProductions(rows);
  assert.equal(text, 'S -> aSb | λ\nA -> a\nS -> c');
  assert.deepEqual(parseProductionText(text).rows, rows);
  assert.equal(serializeProductions([]), '');
  assert.equal(productionText({ lhs: 'S', rhs: '' }), 'S → λ');
  assert.equal(productionText({ lhs: 'S', rhs: 'ab' }), 'S → ab');
});

test('production table: add, remove, move and edit a row without touching the original', () => {
  const rows = [{ lhs: 'S', rhs: 'a' }, { lhs: 'A', rhs: 'b' }, { lhs: 'B', rhs: 'c' }];
  const frozen = JSON.stringify(rows);
  assert.deepEqual(addGrammarRow(rows).at(-1), { lhs: '', rhs: '' });
  assert.deepEqual(addGrammarRow(rows, 1)[1], { lhs: '', rhs: '' });
  assert.deepEqual(removeGrammarRow(rows, 1).map((r) => r.lhs), ['S', 'B']);
  assert.deepEqual(removeGrammarRow(rows, 9), rows);
  assert.deepEqual(moveGrammarRow(rows, 2, -1).map((r) => r.lhs), ['S', 'B', 'A']);
  assert.deepEqual(moveGrammarRow(rows, 0, -1), rows, 'the first row cannot go up');
  assert.deepEqual(moveGrammarRow(rows, 2, 1), rows, 'the last row cannot go down');
  assert.deepEqual(setGrammarRow(rows, 1, { rhs: 'λ' })[1], { lhs: 'A', rhs: '' });
  assert.equal(JSON.stringify(rows), frozen);
});

// ---------------------------------------------------------------- transformations replay

test('grammar replay: the phases run in JFLAP order (λ, unit, useless, then Chomsky)', () => {
  const cnf = transforms['messy-cnf'];
  assert.deepEqual(grammarPhases(cnf.result, cnf.input.productions).map((p) => p.name), ['lambda', 'unit', 'useless', 'cnf']);
  const single = transforms['messy-remove_unit'];
  assert.deepEqual(grammarPhases(single.result, single.input.productions).map((p) => p.name), ['unit']);
});

test('grammar replay: applying the recorded steps to the input gives exactly the grammar the engine returned', () => {
  for (const [name, { input, result }] of Object.entries(transforms)) {
    const phases = grammarPhases(result, input.productions);
    for (const phase of phases) assert.deepEqual(asSet(replayedProductions(phase)), asSet(phase.result), `${name} / ${phase.name}`);
    assert.deepEqual(asSet(phases.at(-1).result), asSet(result.document.productions), `${name}: the last phase is the answer`);
  }
});

test('grammar replay: frames mark what each step removed and added, and end with the phase result', () => {
  const { input, result } = transforms['nullable-remove_lambda'];
  const frames = grammarFrames(result, input.productions);
  const phase = grammarPhases(result, input.productions)[0];
  assert.equal(frames.length, phase.steps.length + 1, 'one frame per step plus the result');
  const expand = frames.find((f) => f.step && f.step.kind === 'expand');
  assert.equal(expand.removed.length, 1);
  assert.equal(expand.productions.filter((p) => p.mark === 'added').length, 6);
  const remove = frames.find((f) => f.step && f.step.kind === 'remove_lambda');
  assert.deepEqual(remove.removed[0], { lhs: 'A', rhs: '' });
  assert.equal(remove.productions.some((p) => p.lhs === 'A' && p.rhs === ''), false);
  const last = frames.at(-1);
  assert.equal(last.step, null);
  assert.equal(last.productions.every((p) => p.mark === 'keep'), true);
  assert.deepEqual(asSet(last.productions), asSet(result.document.productions));
  assert.equal(frames[0].phase, 'lambda');
});

test('grammar replay: Chomsky normal form shows every phase and ends renamed', () => {
  const { input, result } = transforms['messy-cnf'];
  const frames = grammarFrames(result, input.productions);
  assert.deepEqual([...new Set(frames.map((f) => f.phase))], ['lambda', 'unit', 'useless', 'cnf']);
  assert.equal(frames.some((f) => f.step && f.step.kind === 'terminals'), true);
  assert.equal(frames.some((f) => f.step && f.step.kind === 'split'), true);
  assert.deepEqual(asSet(frames.at(-1).productions), asSet(result.document.productions));
  assert.equal(frames.filter((f) => f.step === null).length, 4, 'one result frame per phase');
});

test('grammar replay: steps are described in words with the productions written as A → bc', () => {
  const words = describeGrammarStep({ kind: 'Edge', from: 'S', to: 'A', production: { lhs: 'S', rhs: 'A' } });
  assert.match(words, /S → A/);
  assert.match(describeGrammarStep({ kind: 'DerivesLambda', variable: 'A', production: { lhs: 'A', rhs: '' } }), /A.*λ/);
  assert.match(describeGrammarStep({ kind: 'Split', production: { lhs: 'S', rhs: 'abc' }, replacement: [{ lhs: 'S', rhs: 'aD' }] }), /S → abc/);
  assert.equal(typeof describeGrammarStep({ kind: 'Unheard' }), 'string');
});

test('grammar replay (engine): the invariant also holds for grammars computed right now', async () => {
  const api = await loadApi();
  for (const text of ['S -> ABC | a\nA -> λ | a\nB -> A | b\nC -> λ', 'S -> AB | a\nA -> S | b\nB -> λ\nD -> d', 'S -> aSb | ab | A\nA -> B\nB -> b | λ']) {
    const input = (await api({ op: 'normalize', document: { type: 'grammar', text } })).document;
    for (const to of ['remove_lambda', 'remove_unit', 'remove_useless', 'cnf']) {
      const result = await api({ op: 'convert', document: input, to });
      for (const phase of grammarPhases(result, input.productions)) assert.deepEqual(asSet(replayedProductions(phase)), asSet(phase.result), `${text} / ${to} / ${phase.name}`);
    }
  }
});

// ---------------------------------------------------------------- tables

test('tables: FIRST/FOLLOW rows and the LL(1) table flag the cells with several productions', () => {
  const model = tablesModel(parse.tables);
  assert.equal(model.start, 'E');
  assert.deepEqual(model.firstFollow.map((r) => r.variable), ['E', 'F', 'T']);
  assert.deepEqual(model.firstFollow[0], { variable: 'E', first: ['(', 'i'], follow: ['$', ')', '+'] });
  assert.equal(model.ll1.ok, false);
  const e = model.ll1.rows.find((r) => r.variable === 'E');
  const cell = e.cells.find((c) => c.terminal === '(');
  assert.deepEqual(cell.entries, ['E -> E+T', 'E -> T']);
  assert.equal(cell.conflict, true);
  assert.equal(e.cells.find((c) => c.terminal === '+').entries.length, 0);
  assert.equal(model.ll1.rows.find((r) => r.variable === 'F').cells.every((c) => !c.conflict), true);
  assert.equal(model.ll1.conflicts.length, 4);
  assert.deepEqual(model.ll1.terminals, ['(', ')', '*', '+', 'i', '$']);
});

test('tables: SLR(1) items, ACTION and GOTO per state; conflicts are highlighted', () => {
  const ok = tablesModel(parse.tables).slr1;
  assert.equal(ok.ok, true);
  assert.deepEqual(ok.productions[0], { n: 0, text: "E' -> E" });
  assert.equal(ok.states.length, 12);
  assert.deepEqual(ok.states[2].items, ["E'→E·", 'E→E·+T']);
  assert.equal(ok.states[2].final, true);
  const accept = ok.actionRows[2].cells.find((c) => c.symbol === '$');
  assert.deepEqual(accept.entries, ['acc']);
  assert.equal(ok.gotoRows[0].cells.find((c) => c.variable === 'T').target, 4);
  assert.equal(ok.gotoRows[2].cells.every((c) => c.target === null), true);
  assert.equal(ok.actionRows.flatMap((r) => r.cells).some((c) => c.conflict), false);

  const bad = tablesModel(parse.conflicts).slr1;
  assert.equal(bad.ok, false);
  assert.deepEqual(bad.conflicts, [{ state: 4, symbol: '+', actions: ['r1', 's3'] }]);
  const cell = bad.actionRows[4].cells.find((c) => c.symbol === '+');
  assert.equal(cell.conflict, true);
  assert.deepEqual(cell.entries, ['r1', 's3']);
});

// ---------------------------------------------------------------- parsers

test('parse traces: LL(1) stack/input rows and SLR(1) state/symbol rows with the action in words', () => {
  const ll = ll1Trace(parse.ll1.results[0]);
  assert.equal(ll.length, 8);
  assert.deepEqual(ll[0], { n: 1, stack: 'S', input: 'aabb$', action: 'Replace S → aSb', kind: 'replace' });
  assert.equal(ll[1].action, 'Match a');
  assert.equal(ll[4].action, 'Replace S → λ');
  assert.deepEqual([ll.at(-1).action, ll.at(-1).kind], ['Accept', 'accept']);
  assert.equal(ll1Trace(parse.ll1Rejected.results[0]).at(-1).action, "Reject: Stack and input don't match.");
  const lr = slrTrace(parse.slr1.results[0]);
  assert.deepEqual(lr[0], { n: 1, states: '0', symbols: '', input: 'aabb$', action: 'Shift to state 2', kind: 'shift' });
  assert.equal(lr[2].action, 'Reduce S → λ');
  assert.equal(lr[2].states, '0 2 2');
  assert.equal(describeParseAction('Accept'), 'Accept');
  assert.equal(describeParseAction({ Reduce: { production: { lhs: 'A', rhs: 'ab' }, production_index: 3 } }), 'Reduce A → ab');
});

test('CYK table: the upper triangle of substrings i..j', () => {
  const model = cykModel(parse.cyk.results[0].table, 'ab');
  assert.deepEqual(model.header, ['a', 'b']);
  assert.deepEqual(model.rows.map((r) => r.cells.map((c) => [c.j, c.symbols])), [[[0, ['A']], [1, ['S']]], [[1, ['B']]]]);
  assert.equal(model.rows[0].cells[1].substring, 'ab');
  assert.equal(cykModel({ cells: [[[]]], length: 1 }, 'a').rows[0].cells[0].symbols.length, 0);
});

test('multiple parse: one row per input with its outcome; derivations list the production applied', () => {
  const rows = parse.brute.results.map((r) => [r.input, r.outcome]);
  assert.deepEqual(rows, [['aabb', 'accepted'], ['aab', 'rejected'], ['', 'accepted']]);
  assert.deepEqual(derivationSteps(parse.brute.results[0]).map((s) => s.form), ['aSb', 'aaSbb', 'aabb']);
  assert.deepEqual(derivationSteps(parse.brute.results[0])[0], { n: 1, production: 'S → aSb', form: 'aSb' });
  assert.deepEqual(derivationSteps(parse.brute.results[1]), []);
});

// ---------------------------------------------------------------- user-controlled parse

test('user parse: a choice becomes a move, undo drops the last one, and the choices come from `applicable`', () => {
  assert.deepEqual(userMove({ production: 0 }), { production: 0 });
  assert.deepEqual(userMove({ production: 1, position: 2 }), { production: 1, position: 2 });
  assert.deepEqual(userMove({ production: 1, positions: [0, 3] }), { production: 1, positions: [0, 3] });
  assert.deepEqual(userMove({ production: 1, positions: [4] }), { production: 1, position: 4 }, 'one position is a plain move');
  assert.deepEqual(withoutLastMove([{ production: 0 }, { production: 1 }]), [{ production: 0 }]);
  assert.deepEqual(withoutLastMove([]), []);
  const choices = userChoices(parse.user);
  assert.deepEqual(choices.map((c) => [c.production, c.text, c.positions]), [[0, 'S → aSb', [1]], [1, 'S → λ', [1]]]);
  assert.equal(choices[0].labels[0], 'S at 2', 'positions are shown 1-based on the sentential form');
  assert.equal(choices[0].all, false, 'a single occurrence has no "all" button');
  assert.deepEqual(userChoices(parse.userDone), []);
});

test('parse inputs: one string per line, λ or a blank line is the empty string, trailing blank lines are dropped', () => {
  assert.deepEqual(parseInputLines('aabb\n  ab \nλ\n\n'), ['aabb', 'ab', '']);
  assert.deepEqual(parseInputLines(''), ['']);
  assert.deepEqual(parseInputLines('a\n\nb'), ['a', '', 'b']);
});

test('start variable: the chosen one while it still has productions, otherwise the first left side', () => {
  const rows = [{ lhs: 'S', rhs: 'aA' }, { lhs: 'A', rhs: '' }];
  assert.equal(startVariable(rows, ''), 'S');
  assert.equal(startVariable(rows, 'A'), 'A');
  assert.equal(startVariable(rows, 'Z'), 'S', 'a start variable nothing defines falls back');
  assert.equal(startVariable([{ lhs: '', rhs: 'a' }, ...rows], ''), 'S', 'rows without a left side are not a start');
  assert.equal(startVariable([], 'S'), '');
});
