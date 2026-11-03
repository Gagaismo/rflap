import test from 'node:test';
import assert from 'node:assert/strict';
import { designDefaults, designRequest, designView, designErrorView, constraintMarker, parseExampleLines } from '../src/design-model.js';
import { fixture } from './fixtures.js';
import { loadApi } from './engine.js';
import { setLanguage } from '../src/i18n.js';

setLanguage('en'); // these tests assert the English messages; the app default is Spanish

const design = fixture('design');
const form = (extra = {}) => ({ ...designDefaults(), ...extra });
const message = (f) => {
  try {
    designRequest(f);
  } catch (error) {
    return error.message;
  }
  return null;
};

test('design form: a constraint becomes the spec the engine takes', () => {
  const request = designRequest(form({ alphabet: 'a, b', text: 'ends_with("ab")', accept: 'ab\naab\n\n', reject: 'λ\nba' }));
  assert.deepEqual(request, {
    op: 'design',
    spec: { alphabet: 'ab', language: { constraint_text: 'ends_with("ab")' }, examples: { accept: ['ab', 'aab'], reject: ['', 'ba'] } },
  });
});

test('design form: regex, regular grammar and templates', () => {
  assert.deepEqual(designRequest(form({ mode: 'regex', text: '(a+b)*abb' })).spec.language, { regex: '(a+b)*abb' });
  assert.deepEqual(designRequest(form({ mode: 'regular_grammar', text: 'S -> aS | bA\nA -> λ' })).spec.language, { regular_grammar: 'S -> aS | bA\nA -> λ' });
  const an = designRequest(form({ mode: 'template', template: 'an_bn', a: 'a', b: 'b', min: '1' })).spec.language;
  assert.deepEqual(an, { template: { name: 'an_bn', a: 'a', b: 'b', min: 1 } });
  assert.deepEqual(designRequest(form({ mode: 'template', template: 'an_bm', a: 'a', b: 'b', relation: 'ne' })).spec.language, { template: { name: 'an_bm', a: 'a', b: 'b', relation: 'ne' } });
  assert.deepEqual(designRequest(form({ mode: 'template', template: 'equal_count', a: 'a', b: 'b' })).spec.language, { template: { name: 'equal_count', a: 'a', b: 'b' } });
  assert.deepEqual(designRequest(form({ alphabet: '()[]', mode: 'template', template: 'balanced_brackets', pairs: '() []' })).spec.language, { template: { name: 'balanced_brackets', pairs: [['(', ')'], ['[', ']']] } });
  assert.deepEqual(designRequest(form({ alphabet: 'abc', mode: 'template', template: 'wcwr', c: 'c' })).spec.language, { template: { name: 'wcwr', c: 'c' } });
  assert.deepEqual(designRequest(form({ mode: 'template', template: 'palindrome' })).spec.language, { template: { name: 'palindrome' } });
  assert.deepEqual(designRequest(form({ mode: 'template', template: 'even_palindrome' })).spec.language, { template: { name: 'even_palindrome' } });
});

test('design form: options travel only when the user changed them', () => {
  assert.equal('options' in designRequest(form({ text: 'length >= 1' })), false);
  const request = designRequest(form({ text: 'length >= 1', checkLen: '5', trap: false }));
  assert.deepEqual(request.options, { check_len: 5, include_trap_state: false });
});

test('design form: every mistake has a message that says what to fix', () => {
  assert.match(message(form({ alphabet: '  ,', text: 'x' })), /alphabet/i);
  assert.match(message(form({ text: '   ' })), /constraint/i);
  assert.match(message(form({ mode: 'regex', text: '' })), /regular expression/i);
  assert.match(message(form({ mode: 'regular_grammar', text: '' })), /grammar/i);
  assert.match(message(form({ text: 'length >= 1', accept: 'abc' })), /"abc".*alphabet/);
  assert.match(message(form({ text: 'length >= 1', reject: 'ab\nxb' })), /"xb".*alphabet/);
  assert.match(message(form({ mode: 'template', template: 'an_bn', a: 'a', b: 'c' })), /"c".*alphabet/);
  assert.match(message(form({ mode: 'template', template: 'an_bn', a: 'a', b: 'a' })), /different/i);
  assert.match(message(form({ mode: 'template', template: 'an_bn', a: '', b: 'b' })), /one character/i);
  assert.match(message(form({ mode: 'template', template: 'an_bn', a: 'a', b: 'b', min: '2' })), /0 or 1/);
  assert.match(message(form({ mode: 'template', template: 'balanced_brackets', pairs: '(' })), /pairs/i);
  assert.match(message(form({ text: 'length >= 1', checkLen: '99' })), /length/i);
  assert.equal(message(form({ text: 'length >= 1' })), null);
});

test('design form: example lines (one string per line, λ or "" for the empty string)', () => {
  assert.deepEqual(parseExampleLines('ab\n  aab \n\nλ\n""\r\nba'), ['ab', 'aab', '', '', 'ba']);
  assert.deepEqual(parseExampleLines(''), []);
});

test('design report: states with meanings, regex, stats and the examples that passed or failed', () => {
  const ok = designView(design.ok);
  assert.equal(ok.kind, 'dfa');
  assert.equal(ok.ok, true);
  assert.deepEqual(ok.states.map((s) => [s.name, s.initial, s.accepting]), [['q0', true, false], ['q1', false, false], ['q2', false, true]]);
  assert.equal(ok.states[1].meaning, 'ends with "a", the start of "ab"');
  assert.equal(ok.states[2].accessWord, 'ab');
  assert.equal(ok.regex, '(b*aa*b(aa*b)*b)*b*aa*b(aa*b)*');
  assert.equal(ok.examples.every((e) => e.passed), true);
  assert.equal(ok.crossCheck.checked, 511);
  assert.equal(ok.crossCheck.ok, true);
  assert.deepEqual(ok.accepted.slice(0, 3), ['ab', 'aab', 'bab']);

  const bad = designView(design.failing);
  assert.equal(bad.ok, false);
  assert.deepEqual(bad.failed.map((e) => e.word), ['b']);
  assert.equal(bad.failed[0].expected, true);
  assert.equal(bad.failed[0].actual, false);
  assert.equal(bad.failed[0].detail, 'the constraint rejects this string');
  assert.equal(bad.examples.find((e) => e.word === 'ab').passed, true);
  assert.equal(bad.examples.find((e) => e.word === 'b').passed, false);

  const template = designView(design.template);
  assert.equal(template.kind, 'pda');
  assert.equal(template.regex, null);
  assert.equal(template.acceptance, 'final_state');
});

test('design errors: the position of a constraint syntax error is shown inside the text', () => {
  const error = design.syntaxError.error;
  assert.equal(error.code, 'design_error');
  const view = designErrorView(error, 'ends_with("ab") and ');
  assert.equal(view.position, 20);
  assert.match(view.message, /position 20/);
  assert.deepEqual(constraintMarker('ends_with("ab") and ', 20), { before: 'ends_with("ab") and ', at: '', after: '' });
  assert.deepEqual(constraintMarker('a or )', 5), { before: 'a or ', at: ')', after: '' });
  assert.deepEqual(constraintMarker('abc', 1), { before: 'a', at: 'b', after: 'c' });
  const plain = designErrorView({ code: 'invalid_regex', message: 'bad', details: { position: 4 } }, 'x');
  assert.equal(plain.message, 'bad');
  assert.equal(plain.position, null, 'only constraint syntax errors point into the text');
});

test('design (engine): the request the form builds is accepted by the engine', async () => {
  const api = await loadApi();
  const request = designRequest(form({ alphabet: 'ab', text: 'count(a) % 2 == 0 and ends_with("b")', accept: 'aab\nabab', reject: 'a\nab' }));
  const result = await api(request);
  assert.equal(designView(result).states.length, 3);
  assert.equal(designView(result).ok, true);
  const grammar = await api(designRequest(form({ mode: 'regular_grammar', text: 'S -> aS | bA\nA -> bA | λ', reject: 'a' })));
  assert.equal(designView(grammar).kind, 'dfa');
  const bracket = await api(designRequest(form({ alphabet: '()', mode: 'template', template: 'balanced_brackets', pairs: '()', accept: '(())' })));
  assert.equal(designView(bracket).kind, 'pda');
});
