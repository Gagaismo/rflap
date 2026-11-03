import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { registerPanel, panelsFor, registerAction, actionsFor, actionById } from '../src/registry.js';
import '../src/sim.js';
import '../src/structure.js';
import '../src/analysis.js';
import '../src/steps.js';
import '../src/assistant.js';
import '../src/info.js';
import '../src/actions.js';
import '../src/grammar-editor.js';
import '../src/grammar-transform.js';
import '../src/grammar-tables.js';
import '../src/grammar-parse.js';
import '../src/grammar-membership.js';
import '../src/catedra-panels.js';
import '../src/regex-panel.js';
import '../src/design-panel.js';
import '../src/lsystem-panel.js';
import '../src/pumping-panel.js';
import { dictionaries } from '../src/i18n.js';
import { collectStateIds, resolveStep, mapIds, spaceOf } from '../src/steps.js';

const src = path.join(path.dirname(fileURLToPath(import.meta.url)), '../src');

test('the built-in panels apply to the right document types, in order', () => {
  assert.deepEqual(panelsFor('fa').map((p) => p.id), ['simulate', 'multiple', 'structure', 'analysis', 'design', 'document']);
  const tab = { steps: { steps: [{}] }, extras: { svg: '<svg/>' } };
  assert.deepEqual(panelsFor('turing', tab).map((p) => p.id), ['assistant', 'simulate', 'multiple', 'structure', 'catedra-definition', 'catedra-table', 'catedra-run', 'catedra-all', 'analysis', 'steps', 'design', 'document']);
});

test('every JFLAP environment has its own panels (T12b)', () => {
  const ids = (type, tab) => panelsFor(type, tab).map((p) => p.id);
  assert.deepEqual(ids('grammar'), ['grammar-editor', 'grammar-transform', 'grammar-tables', 'grammar-parse', 'grammar-membership', 'grammar-user', 'design', 'document']);
  assert.deepEqual(ids('regex'), ['regex-editor', 'design', 'document']);
  assert.deepEqual(ids('lsystem'), ['lsystem-editor', 'design', 'document']);
  assert.deepEqual(ids('regular_pumping_lemma'), ['pumping-game', 'design', 'document']);
  assert.deepEqual(ids('context_free_pumping_lemma'), ['pumping-game', 'design', 'document']);
  assert.equal(ids('grammar', { steps: { steps: [{}] } }).includes('steps'), true, 'conversion results of a grammar tab replay too');
  assert.equal(ids('fa').includes('grammar-editor'), false);
});

test('the File menu offers a new document of every environment, and the design panel', () => {
  const file = actionsFor('file', 'fa').map((a) => a.id);
  for (const id of ['file.new.fa', 'file.new.grammar', 'file.new.regex', 'file.new.lsystem', 'file.new.pumping_regular', 'file.new.pumping_cf', 'file.new.design']) assert.ok(file.includes(id), id);
  assert.ok(file.indexOf('file.new.design') < file.indexOf('file.open'), 'the New entries come before Open');
  for (const type of ['grammar', 'regex', 'lsystem', 'regular_pumping_lemma']) assert.ok(actionsFor('file', type).some((a) => a.id === 'file.new.grammar'), type);
});

test('the menus of a grammar: transformations, parsers and conversions; automata keep their own', () => {
  const ids = (menu, type) => actionsFor(menu, type).map((a) => a.id);
  assert.deepEqual(ids('convert', 'grammar').filter((id) => id.startsWith('convert.step.')), ['convert.step.remove_lambda', 'convert.step.remove_unit', 'convert.step.remove_useless', 'convert.step.cnf']);
  for (const to of ['minimal_dfa', 'fa', 'pda_ll', 'pda_lr']) assert.ok(ids('convert', 'grammar').some((id) => id.startsWith(`convert.${to}.`)), to);
  assert.deepEqual(ids('input', 'grammar'), ['input.tables', 'input.parse.brute', 'input.parse.ll1', 'input.parse.slr1', 'input.parse.cyk', 'input.parse.user', 'input.membership_tk']);
  assert.ok(ids('convert', 'regex').includes('convert.equivalent.regex'));
  assert.equal(ids('input', 'regex').length, 0, 'a regular expression has no Input menu');
  assert.ok(ids('input', 'fa').includes('input.run'));
  assert.equal(ids('convert', 'grammar').includes('convert.cnf.grammar'), false, 'the Chomsky entry opens the replay instead');
});

test('Turing machines get the cátedra panels and menu entries; other machines do not', () => {
  const ids = (menu, type) => actionsFor(menu, type).map((a) => a.id);
  assert.ok(ids('file', 'fa').includes('file.new.all') && ids('file', 'fa').includes('file.new.turing'), 'MT and ALL are in File > New');
  assert.ok(ids('file', 'fa').indexOf('file.new.all') < ids('file', 'fa').indexOf('file.open'));
  assert.deepEqual(ids('view', 'turing').filter((id) => id.startsWith('view.notation.')), ['view.notation.catedra', 'view.notation.jflap']);
  assert.equal(ids('view', 'fa').some((id) => id.startsWith('view.notation.')), false);
  assert.ok(ids('test', 'turing').includes('test.all_check') && !ids('test', 'fa').includes('test.all_check'));
  assert.ok(ids('convert', 'turing').includes('convert.compose_tm') && !ids('convert', 'fa').includes('convert.compose_tm'));
  assert.ok(ids('input', 'turing').includes('input.catedra_run'));
  for (const id of ['catedra-definition', 'catedra-table', 'catedra-run', 'catedra-all']) assert.equal(panelsFor('fa').some((p) => p.id === id), false, id);
});

test('T12b can register panels and actions for new types', () => {
  registerPanel({ id: 'test-grammar', title: 'Grammar', appliesTo: ['grammar'], order: 5, mount() {} });
  assert.equal(panelsFor('grammar')[0].id, 'test-grammar');
  assert.equal(panelsFor('fa').some((p) => p.id === 'test-grammar'), false);
  registerPanel({ id: 'test-hidden', title: 'Hidden', appliesTo: ['*'], visible: (t) => Boolean(t && t.show === true), mount() {} });
  assert.equal(panelsFor('fa', { show: false }).some((p) => p.id === 'test-hidden'), false);
  assert.equal(panelsFor('fa', { show: true }).some((p) => p.id === 'test-hidden'), true);
  registerAction({ id: 'test.action', menu: 'convert', label: 'Test', appliesTo: ['grammar'], order: -1, run() {} });
  assert.equal(actionsFor('convert', 'grammar')[0].id, 'test.action');
  assert.equal(actionById('test.action').menu, 'convert');
  assert.throws(() => registerPanel({ id: 'bad' }), /mount/);
  assert.throws(() => registerAction({ id: 'bad' }), /menu/);
});

test('actions exist for the JFLAP menus and the types they apply to', () => {
  const ids = (menu, type) => actionsFor(menu, type).map((a) => a.id);
  assert.ok(ids('file', 'fa').includes('file.open') && ids('file', 'fa').includes('file.save_jff') && ids('file', 'fa').includes('file.share'));
  assert.deepEqual(ids('layout', 'pda'), ['layered', 'circle', 'gem', 'random', 'spiral', 'tree', 'tree-degree', 'two-circle'].map((a) => `layout.${a}`));
  assert.ok(ids('convert', 'fa').includes('convert.combine') && ids('convert', 'fa').includes('convert.equivalent'));
  assert.equal(ids('convert', 'pda').includes('convert.combine'), false);
  assert.ok(ids('input', 'fa').includes('input.step_closure') && ids('input', 'turing').includes('input.step'));
  assert.equal(ids('input', 'turing').includes('input.step_closure'), false);
  assert.ok(ids('test', 'fa').includes('test.nondeterminism') && ids('test', 'fa').includes('test.lambda'));
  assert.equal(actionsFor('layout', 'grammar').length, 0);
});

test('every literal i18n key used in the sources exists in English and Spanish', () => {
  const missing = [];
  for (const file of fs.readdirSync(src).filter((f) => f.endsWith('.js') && f !== 'i18n.js')) {
    const text = fs.readFileSync(path.join(src, file), 'utf8');
    for (const [, key] of text.matchAll(/\bt\('([a-z0-9_.-]+)'/g)) {
      for (const locale of ['en', 'es']) if (!(key in dictionaries[locale])) missing.push(`${file}: ${locale} ${key}`);
    }
    for (const [, key] of text.matchAll(/\b(?:label|title):\s*'((?:file|edit|input|test|convert|view|help|block|panel|layout)\.[a-z0-9_.-]+)'/g)) {
      for (const locale of ['en', 'es']) if (!(key in dictionaries[locale])) missing.push(`${file}: ${locale} ${key}`);
    }
  }
  assert.deepEqual(missing, []);
});

test('dynamic i18n keys exist: menus, types, tools, layouts, conversions, simulation options', () => {
  const keys = [
    ...['file', 'edit', 'input', 'test', 'convert', 'layout', 'view', 'help'].map((m) => `menu.${m}`),
    ...['fa', 'pda', 'turing', 'mealy', 'moore', 'grammar', 'regex', 'lsystem', 'regular_pumping_lemma', 'context_free_pumping_lemma'].map((x) => `type.${x}`),
    ...['select', 'state', 'transition', 'delete'].flatMap((x) => [`tool.${x}`, `tool.hint.${x}`]),
    ...['closure', 'step_by_state'].map((x) => `sim.mode.${x}`),
    ...['final_state', 'empty_stack', 'halting', 'both'].map((x) => `sim.acceptance.${x}`),
    ...['dfa', 'nfa', 'dpda', 'npda', 'turing', 'mealy', 'moore'].map((x) => `analysis.kind.${x}`),
    ...['accepted', 'rejected', 'undecided'].map((x) => `sim.${x}`),
    ...['tools', 'delete', 'rename', 'arrows', 'undo', 'open', 'zoom', 'step', 'run', 'menu'].flatMap((x) => [`help.sc.${x}.keys`, `help.sc.${x}.what`]),
    ...['lambda', 'unit', 'useless'].map((x) => `transform.${x}`),
    ...['lambda', 'unit', 'useless', 'cnf'].map((x) => `transform.phase.${x}`),
    ...['added', 'focus'].map((x) => `transform.mark.${x}`),
    ...['derives_lambda', 'remove_lambda', 'expand', 'edge', 'reach', 'remove_unit', 'add', 'derives_terminals', 'remove_non_deriving', 'reachable', 'remove_unreachable', 'terminals', 'split'].map((x) => `grammar.step.${x}`),
    ...['accepted', 'rejected', 'error'].map((x) => `parse.outcome.${x}`),
    ...['brute', 'll1', 'slr1', 'cyk'].map((x) => `parse.parser.${x}`),
    ...['constraint_text', 'regex', 'regular_grammar', 'template'].flatMap((x) => [`design.mode.${x}`]),
    ...['constraint_text', 'regex', 'regular_grammar'].map((x) => `design.text.${x}`),
    ...['an_bn', 'an_bm', 'equal_count', 'balanced_brackets', 'wcwr', 'even_palindrome', 'palindrome'].map((x) => `design.template.${x}`),
    ...['le', 'ge', 'ne'].map((x) => `design.template.relation.${x}`),
    ...['dfa', 'pda', 'nfa'].map((x) => `design.kind.${x}`),
    ...['computer_first', 'human_first'].map((x) => `pumping.mode.${x}`),
    ...['not_regular', 'not_context_free'].map((x) => `pumping.conclusion.${x}`),
  ];
  for (const key of keys) for (const locale of ['en', 'es']) assert.ok(key in dictionaries[locale], `${locale} ${key}`);
  // every label of every registered action is a key (or a plain name such as a language)
  for (const menu of ['file', 'edit', 'input', 'test', 'convert', 'layout', 'view', 'help']) {
    for (const a of actionsFor(menu)) if (a.label.includes('.')) assert.ok(a.label in dictionaries.en && a.label in dictionaries.es, a.label);
  }
});

test('conversion steps: ids are classified as source or result and resolved to names', () => {
  const merge = { kind: 'merge_group', group: [0, 3], state: 1 };
  assert.deepEqual(collectStateIds(merge), { source: [0, 3], result: [1] });
  assert.deepEqual(resolveStep(merge, (id) => `s${id}`, (id) => `r${id}`), { kind: 'merge_group', group: ['s0', 's3'], state: 'r1' });
  assert.deepEqual(collectStateIds({ kind: 'new_state', nfa_states: [0, 2], state: 5 }), { source: [0, 2], result: [5] });
  assert.deepEqual(collectStateIds({ kind: 'eliminate_state', state: 1, edges: [{ from: 0, label: 'a', to: 2 }] }), { source: [1, 0, 2], result: [] });
  assert.deepEqual(collectStateIds({ action: 'decat', new_states: [2, 3], transition: { from: 0, to: 1, label: 'ab' } }), { source: [], result: [2, 3, 0, 1] });
  assert.deepEqual(collectStateIds({ kind: 'add_trap_state', state: 'q3', transitions_added: 6 }), { source: [], result: [] });
  assert.equal(spaceOf({ kind: 'split' }, 'group'), 'source');
  const same = { kind: 'result', expression: 'a*b', count: 3 };
  assert.deepEqual(mapIds(same, () => 'X'), same);
});
