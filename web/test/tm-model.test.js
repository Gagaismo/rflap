import fs from 'node:fs';
import test from 'node:test';
import assert from 'node:assert/strict';
import { fixture } from './fixtures.js';
import { loadApi } from './engine.js';
import { Tab } from '../src/tab.js';
import { emptyDocument, labelToText, labelFromText } from '../src/doc.js';
import { fieldsFromText, textFromFields } from '../src/labels.js';
import {
  inferModel, modelOf, labelText, parseLabelText, formValues, textFromForm, moveLetters, showMove, normalizeMove, blankOf, notationOf,
  sequenceView, usesAsciiDelimiters, convertAsciiDelimiters, inputHasDelimiters, acceptanceUsed, catedraBatchRequest, catedraRows, stateOfConfiguration, statesOfRun, definitionRows, formatSet, violationTransitions, tkRows, effectiveInputs, multipistaOf,
} from '../src/tm-model.js';

const f = fixture('catedra');
const palindromeNames = Object.values(f.palindrome.automaton.states).map((s) => s.name);
const igualNames = Object.values(f.igual.automaton.states).map((s) => s.name);

// ---------------------------------------------------------------- labels

test('a label is written in the cátedra notation with Δ and I / N / D, or in the JFLAP one with □ and L / S / R', () => {
  const label = { actions: [{ read: 'a', write: '□', direction: 'right' }] };
  assert.equal(labelText(label, 'catedra'), '(a,Δ,D)');
  assert.equal(labelText(label, 'jflap'), 'a;□,R');
  assert.equal(labelText({ actions: [{ read: '□', write: '□', direction: 'left' }] }, 'catedra'), '(Δ,Δ,I)');
  assert.equal(labelText({ actions: [{ read: 'X', write: 'X', direction: 'stay' }] }, 'catedra'), '(X,X,N)');
  assert.equal(labelToText('turing', label, 'catedra'), '(a,Δ,D)');
  assert.equal(labelToText('turing', label), 'a;_,R', 'without a notation the old JFLAP text stays');
});

test('several tapes keep the JFLAP form with Δ (the course shows them that way)', () => {
  const label = { actions: [{ read: 'a', write: 'b', direction: 'right' }, { read: '□', write: 'x', direction: 'stay' }] };
  assert.equal(labelText(label, 'catedra'), 'a;b,R|Δ;x,S');
  assert.equal(labelText(label, 'jflap'), 'a;b,R|□;x,S');
});

test('a read pattern with commas falls back to the JFLAP form', () => {
  assert.equal(labelText({ actions: [{ read: 'a,b}v', write: 'x', direction: 'right' }] }, 'catedra'), 'a,b}v;x,R');
});

test('parseLabelText reads (a,X,D), a,X,D, a;X,R and several tapes; Δ □ and _ are the blank', () => {
  assert.deepEqual(parseLabelText('(a,X,D)'), [{ read: 'a', write: 'X', move: 'R' }]);
  assert.deepEqual(parseLabelText('a,X,I'), [{ read: 'a', write: 'X', move: 'L' }]);
  assert.deepEqual(parseLabelText('(Δ,Δ,N)'), [{ read: '', write: '', move: 'S' }]);
  assert.deepEqual(parseLabelText('a;X,R'), [{ read: 'a', write: 'X', move: 'R' }]);
  assert.deepEqual(parseLabelText('_;□,L'), [{ read: '', write: '', move: 'L' }]);
  assert.deepEqual(parseLabelText('a;b,R|c;d,L'), [{ read: 'a', write: 'b', move: 'R' }, { read: 'c', write: 'd', move: 'L' }]);
  for (const bad of ['', '(a,b)', '(a,b,X)', 'a;b', 'a,b,c,d']) assert.equal(parseLabelText(bad), null, bad);
});

test('the move letters are I / N / D for one tape in the cátedra notation and L / S / R otherwise', () => {
  assert.deepEqual(moveLetters('catedra', 1), ['I', 'N', 'D']);
  assert.deepEqual(moveLetters('jflap', 1), ['L', 'S', 'R']);
  assert.deepEqual(moveLetters('catedra', 2), ['L', 'S', 'R']);
  assert.equal(normalizeMove('D'), 'R');
  assert.equal(normalizeMove('I'), 'L');
  assert.equal(normalizeMove('N'), 'S');
  assert.equal(normalizeMove('left'), 'L');
  assert.equal(normalizeMove('x'), null);
  assert.equal(showMove('R', 'catedra', 1), 'D');
  assert.equal(showMove('R', 'jflap', 1), 'R');
  assert.equal(blankOf('catedra'), 'Δ');
  assert.equal(blankOf('jflap'), '□');
  assert.equal(notationOf('latex'), 'catedra');
});

test('the form: fields -> text -> fields round-trips in both notations, incl. Δ and the typed label', () => {
  assert.equal(textFromForm({ read0: 'a', write0: '', move0: 'D' }, 'catedra', 1), '(a,Δ,D)');
  assert.deepEqual(formValues('(a,Δ,D)', 'catedra', 1), { read0: 'a', write0: '', move0: 'D' }, 'the blank is an empty field (its placeholder shows Δ)');
  assert.equal(textFromForm({ read0: 'a', write0: '', move0: 'R' }, 'jflap', 1), 'a;□,R');
  assert.deepEqual(formValues('a;□,R', 'jflap', 1), { read0: 'a', write0: '', move0: 'R' });
  assert.deepEqual(formValues('(a,X,D)', 'jflap', 1), { read0: 'a', write0: 'X', move0: 'R' }, 'a label typed in the other notation reads back');
  assert.deepEqual(formValues('a;b,R|c;d,L', 'catedra', 2), { read0: 'a', write0: 'b', move0: 'R', read1: 'c', write1: 'd', move1: 'L' });
  assert.equal(textFromForm({ read0: 'a', write0: 'b', move0: 'R', read1: '', write1: 'x', move1: 'S' }, 'catedra', 2), 'a;b,R|Δ;x,S');
  assert.equal(textFromForm({ read0: 'z', write0: 'z', move0: 'D', full: ' (a,X,I) ' }, 'catedra', 1), '(a,X,I)', 'the typed label wins');
  assert.deepEqual(fieldsFromText('turing', '(a,X,D)', 1, 'catedra'), { read0: 'a', write0: 'X', move0: 'D' });
  assert.equal(textFromFields('turing', { read0: 'a', write0: 'X', move0: 'I' }, 1, 'catedra'), '(a,X,I)');
  assert.equal(textFromFields('fa', { text: ' ab ' }), 'ab', 'other types are untouched');
});

test('the engine accepts every text the form produces', async () => {
  const api = await loadApi();
  for (const [text, notation, tapes] of [['(a,Δ,D)', 'catedra', 1], ['a;□,R', 'jflap', 1], ['a;Δ,R|Δ;x,S', 'catedra', 2], ['(Δ,Δ,I)', 'catedra', 1]]) {
    const label = await labelFromText(api, 'turing', text, tapes);
    assert.equal(labelText(label, notation), text, text);
  }
  const typed = await labelFromText(api, 'turing', textFromForm({ full: '(a,X,D)' }, 'catedra', 1), 1);
  assert.deepEqual(typed.actions, [{ direction: 'right', read: 'a', write: 'X' }]);
});

test("the engine draws a Turing machine in the cátedra notation unless JFLAP's is asked for", async () => {
  const api = await loadApi();
  const labels = (svg) => [...svg.matchAll(/<text class="label"[^>]*>([^<]*)</g)].map((m) => m[1]);
  assert.deepEqual(labels(f.svg).slice(0, 3), ['(a,Δ,D)', '(b,Δ,D)', '(Δ,Δ,D)']);
  const jflap = await api({ op: 'render', document: f.palindrome, format: 'svg', notation: 'jflap' });
  assert.deepEqual(labels(jflap.content).slice(0, 2), ['a ; □ , R', 'b ; □ , R']);
  const joined = await api({ op: 'render', document: f.palindrome, format: 'svg', join_labels: true });
  assert.ok(labels(joined.content).includes('(a,a,D) / (b,b,D)'));
});

// ---------------------------------------------------------------- model

test('the model is inferred from ⟨ and ⟩ and remembered per tab', () => {
  assert.equal(inferModel(f.palindrome), 'mt');
  assert.equal(inferModel(f.igual), 'all');
  assert.equal(inferModel(emptyDocument('turing')), 'mt');
  assert.equal(inferModel(emptyDocument('fa')), 'mt');
  assert.equal(inferModel(null), 'mt');
  assert.equal(modelOf('all'), 'all');
  assert.equal(modelOf('xyz'), 'mt');
  const app = { api: async () => ({}), host: { mode: 'standalone' }, t: (k) => k, notify() {}, tabs: () => [], openTab() {} };
  const tab = new Tab(app, { doc: f.igual, title: 'IGUAL' });
  assert.equal(tab.model, 'all', 'a document that arrives with ⟨ ⟩ is an ALL');
  tab.setModel('mt');
  assert.equal(tab.model, 'mt', 'the choice is kept with the tab');
  const heard = [];
  tab.on('model', (m) => heard.push(m));
  tab.setModel('all');
  assert.deepEqual(heard, ['all']);
  tab.replace(f.palindrome, 'host');
  assert.equal(tab.model, 'mt', 'a new document re-infers the model');
  const blank = new Tab(app, { doc: emptyDocument('turing'), title: 'ALL' });
  blank.setModel('all');
  assert.equal(blank.model, 'all', 'a new empty ALL stays an ALL');
});

// ---------------------------------------------------------------- runs

test('the palindrome run view: configurations, verdict class and the tail the engine wrote', () => {
  const [aba, ab, bb, empty] = f.palindromeRuns.results.map(sequenceView);
  assert.equal(aba.configs.length, 11);
  assert.equal(aba.configs[0], 'Δ1abaΔ');
  assert.equal(aba.tail, 'final/acepta');
  assert.equal(aba.verdictClass, 'accept');
  assert.equal(aba.accepted, true);
  assert.equal(aba.steps, 10);
  assert.equal(ab.tail, 'bloqueo/rechazo');
  assert.equal(ab.verdictClass, 'reject');
  assert.equal(ab.accepted, false);
  assert.equal(bb.verdictClass, 'accept');
  assert.equal(empty.input, '');
  assert.equal(empty.configs.join(' ├─ '), 'Δ1Δ ├─ ΔΔ8Δ');
  for (const v of [aba, ab, bb, empty]) assert.equal(`${v.configs.join(' ├─ ')} ${v.tail}`, v.text);
});

test('the IGUAL run view: the whole tape between ⟨ and ⟩, accepts and rejects', () => {
  const [ab, b] = f.igualRuns.results.map(sequenceView);
  assert.equal(ab.configs[1], 'A⟨Xb⟩');
  assert.equal(ab.tail, 'final/acepta');
  assert.equal(ab.verdictClass, 'accept');
  assert.equal(b.text, '⟨Ib⟩ ├─ ⟨bI⟩ ├─ ⟨Db⟩ bloqueo/rechaza');
  assert.equal(b.verdictClass, 'reject');
});

test('a run that does not halt is amber and an ALL violation is explained', () => {
  const loop = sequenceView(f.loopRun.results[0]);
  assert.equal(loop.verdictClass, 'nohalt');
  assert.match(loop.tail, /^no se detiene/);
  const broken = sequenceView(f.brokenRun.results[0]);
  assert.equal(broken.verdictClass, 'violation');
  assert.match(broken.tail, /^violación del ALL/);
  assert.equal(broken.violation.kind, 'sale_por_derecha');
  assert.equal(broken.violation.transition, 1);
});

test('the state of each configuration, to highlight it on the canvas', () => {
  const run = sequenceView(f.palindromeRuns.results[0]);
  assert.deepEqual(statesOfRun(run.configs, palindromeNames, '1'), ['1', '2', '2', '2', '3', '4', '4', '1', '5', '6', '8']);
  const igual = sequenceView(f.igualRuns.results[0]);
  assert.deepEqual(statesOfRun(igual.configs, igualNames, 'I'), ['I', 'A', 'B', 'B', 'C', 'C', 'I', 'I', 'I', 'D', 'D', 'D', 'F']);
  assert.equal(stateOfConfiguration('ΔΔb2aΔ', palindromeNames).name, '2');
  assert.equal(stateOfConfiguration('abc', palindromeNames), null);
});

test('a tape symbol that looks like a state name is told apart by the previous step', () => {
  // states "a" and "p": in "ΔapΔ" the state is p and a is a tape symbol
  assert.deepEqual(statesOfRun(['ΔpaΔ', 'ΔapΔ', 'ΔaΔpΔ'], ['a', 'p'], 'p'), ['p', 'p', 'p']);
});

test('the inputs of a run: blank rows are skipped, λ is the empty word', () => {
  assert.deepEqual(effectiveInputs(['aba', '', ' bb ', 'λ']), ['aba', 'bb', '']);
  assert.deepEqual(effectiveInputs(['']), []);
  assert.deepEqual(effectiveInputs([''], { fallback: true }), ['']);
  assert.deepEqual(effectiveInputs(['ab'], { fallback: true }), ['ab']);
});

// ---------------------------------------------------------------- definition, ALL check, multipista, Tk

test('the formal definition rows, with the sets as the course writes them', () => {
  const view = definitionRows(f.palindromeDefinition);
  assert.equal(view.header, 'MT = ⟨Q, Σ_E, Σ_A, q0, F, f⟩');
  assert.deepEqual(view.rows.map((r) => r.symbol), ['Q', 'Σ_E', 'Σ_A', 'q0', 'F']);
  assert.equal(view.rows[0].set, '{1, 2, 3, 4, 5, 6, 7, 8}');
  assert.equal(view.rows[1].set, '{a, b}');
  assert.equal(view.rows[4].set, '{8}');
  assert.equal(f.palindromeDefinition.input_alphabet_inferred, true);
  const all = definitionRows(f.igualDefinition);
  assert.equal(all.header, 'ALL = ⟨Q, Σ_E, Σ_A, q0, F, f⟩');
  assert.equal(all.rows[2].set, `{${f.igualDefinition.auxiliary_alphabet.join(', ')}}`);
  assert.equal(formatSet([]), '∅');
});

test('the ALL check lists its violations and the transitions to highlight', () => {
  assert.equal(f.igualCheck.ok, true);
  assert.deepEqual(violationTransitions(f.igualCheck.violations), []);
  assert.equal(f.brokenCheck.ok, false);
  const staticOnes = f.brokenCheck.violations.filter((v) => v.input === undefined);
  const dynamicOnes = f.brokenCheck.violations.filter((v) => v.input !== undefined);
  assert.ok(staticOnes.length >= 2 && dynamicOnes.length >= 1);
  assert.deepEqual(violationTransitions(f.brokenCheck.violations), [1, 2]);
});

test('multipista is reported for machines of several tapes only', () => {
  assert.equal(multipistaOf(f.analysisTwoTapes).multipista, false);
  assert.deepEqual(multipistaOf(f.analysisTwoTapes).offending, [1]);
  assert.equal(multipistaOf({ multipista: { applicable: false } }), null);
  assert.equal(multipistaOf({}), null);
});

test('T0 .. Tk of the bibliography example and the contracting grammar', () => {
  const rows = tkRows(f.membership.report);
  assert.deepEqual(rows.map((r) => r.name), ['T0', 'T1', 'T2', 'T3', 'T4']);
  assert.equal(rows[0].text, '{S}');
  assert.equal(rows[3].text, '{S, T, aTBD, abD, aTBc, abc}');
  assert.equal(rows[4].text, 'T3');
  assert.equal(rows[4].same, true);
  assert.equal(f.membership.report.member, false);
  assert.equal(f.membership.report.bound, 4);
  assert.equal(f.membershipContracting.error.code, 'contracting_grammar');
  assert.equal(tkRows({ sets: [['S'], ['S', '']] })[1].text, '{S, λ}');
});

test('the composition of two machines is a Turing document with its definition', () => {
  assert.equal(f.composed.document.type, 'turing');
  assert.equal(inferModel(f.composed.document), 'mt');
  assert.equal(f.composed.definition.model, 'mt');
});

// ---------------------------------------------------------------- all-shrink: delimiters, criterion, notes (T2)

const lt = (read, write, direction = 'right') => ({ from: 0, to: 0, label: { actions: [{ read, write, direction }] } });
const machine = (...transitions) => ({ type: 'turing', automaton: { states: {}, transitions } });

test('S7: a machine with both ASCII < and > (and no ⟨ ⟩) converts them; one alone stays a symbol', () => {
  const both = machine(lt('a', '<'), lt('c', '>', 'left'));
  assert.equal(usesAsciiDelimiters(both), true);
  const converted = convertAsciiDelimiters(both);
  assert.deepEqual(converted.automaton.transitions.map((t) => t.label.actions[0].write), ['⟨', '⟩']);
  assert.equal(both.automaton.transitions[0].label.actions[0].write, '<', 'the input document is not mutated');
  const one = machine(lt('a', '<'), lt('c', 'c'));
  assert.equal(usesAsciiDelimiters(one), false);
  assert.equal(convertAsciiDelimiters(one), one);
  const mixed = machine(lt('a', '⟨'), lt('<', '>'));
  assert.equal(convertAsciiDelimiters(mixed), mixed, 'a machine that already has ⟨ ⟩ is left alone');
});

test('S2/S7: ASCII < > count as ALL when inferring the model', () => {
  assert.equal(inferModel(machine(lt('a', '<'), lt('c', '>'))), 'all');
  assert.equal(inferModel(machine(lt('a', '<'))), 'mt');
  assert.equal(inferModel(machine(lt('a', '⟨'))), 'all');
});

test('S4: an input that already brings the delimiters is detected (⟨aba⟩ and <aba>)', () => {
  for (const x of ['<aba>', '⟨aba⟩', ' <a> ', '<>', '⟨a>']) assert.equal(inputHasDelimiters(x), true, x);
  for (const x of ['aba', '<aba', 'aba>', '', '<', 'a<b>c']) assert.equal(inputHasDelimiters(x), false, x);
});

test('S3: an ALL always accepts by final state; an MT keeps the chosen criterion', () => {
  assert.equal(acceptanceUsed('all', 'halting'), 'final_state');
  assert.equal(acceptanceUsed('mt', 'halting'), 'halting');
  assert.equal(acceptanceUsed('mt', 'both'), 'both');
  assert.equal(acceptanceUsed('mt', undefined), 'final_state');
});

test('S2: the multiple run of an ALL asks catedra_run with model all and the panel\'s max steps', () => {
  const doc = { type: 'turing' };
  assert.deepEqual(catedraBatchRequest(doc, ['aba', ''], 'all', 500), { op: 'catedra_run', document: doc, inputs: ['aba', ''], model: 'all', max_steps: 500 });
});

test('S3/S4: catedra_run results map to multiple-run rows with outcome, notes and violation message', () => {
  const rows = catedraRows([
    { verdict: 'accepted', accepted: true, notes: ['ℹ️ q0 escribe ⟨ sobre a: la cinta se achica'] },
    { verdict: 'rejected', accepted: false },
    { verdict: 'no_halt', accepted: false },
    { verdict: 'violation', accepted: false, violation: { message: 'cruza ⟨' } },
  ]);
  assert.deepEqual(rows.map((r) => r.outcome), ['accepted', 'rejected', 'undecided', 'rejected']);
  assert.deepEqual(rows[0].notes, ['ℹ️ q0 escribe ⟨ sobre a: la cinta se achica']);
  assert.deepEqual(rows[1].notes, []);
  assert.equal(rows[3].message, 'cruza ⟨');
  assert.deepEqual(sequenceView({ verdict: 'accepted', notes: ['x'] }).notes, ['x']);
  assert.deepEqual(sequenceView({ verdict: 'accepted' }).notes, []);
});

test('S3: the tab commit stores ⟨ ⟩ (never ASCII) and switches to ALL', async () => {
  const tab = new Tab({ api: async () => ({}), notify() {} }, { doc: machine(lt('a', 'a')), title: 'x' });
  assert.equal(tab.model, 'mt');
  await tab.edit((d) => d.automaton.transitions.push(lt('a', '<'), lt('c', '>', 'left')));
  assert.deepEqual(tab.doc.automaton.transitions.map((t) => t.label.actions[0].write), ['a', '⟨', '⟩']);
  assert.equal(tab.model, 'all');
});

test('S3: b) multiple run of aba, abab, babab, aaa, a, b, bb accepts only abab (1 of 7)', async () => {
  const api = await loadApi();
  const xml = fs.readFileSync(new URL('../../crates/rflap-api/tests/fixtures/catedra/ALL_2.1b_complemento_palindromo.jff', import.meta.url), 'utf8');
  const { document } = await api({ op: 'open_jff', xml });
  assert.equal(inferModel(document), 'all', 'the ⟨ ⟩ of the .jff make the tab run as ALL');
  const inputs = ['aba', 'abab', 'babab', 'aaa', 'a', 'b', 'bb'];
  const r = await api(catedraBatchRequest(document, inputs, 'all', 1000));
  const rows = catedraRows(r.results);
  assert.ok(rows.every((x) => !x.message), 'the shrink is a note, never a violation');
  assert.deepEqual(rows.map((x) => x.accepted), inputs.map((w) => w === 'abab'));
  assert.equal(rows.filter((x) => x.accepted).length, 1);
});
