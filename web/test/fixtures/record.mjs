// Records the engine answers the model tests replay: `node web/test/fixtures/record.mjs`.
// The fixtures are small, real answers of dist/rflap.html's wasm; rerun it when the api changes.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { loadApi } from '../engine.js';

const dir = path.dirname(fileURLToPath(import.meta.url));
const api = await loadApi();
const save = (name, value) => fs.writeFileSync(path.join(dir, `${name}.json`), `${JSON.stringify(value, null, 1)}\n`);

const grammar = (text) => ({ type: 'grammar', text });
const nfa = { type: 'fa', initial: 'q0', finals: ['q3'], transitions: [['q0', 'a', 'q0'], ['q0', 'b', 'q0'], ['q0', 'a', 'q1'], ['q1', 'b', 'q2'], ['q2', 'b', 'q3']] };
const expr = 'E -> E+T | T\nT -> T*F | F\nF -> (E) | i';

// grammars: the transformations with their steps (and the grammar they started from)
const messy = grammar('S -> aSb | A | λ\nA -> a | B\nC -> c');
const nullable = grammar('S -> ABA | b\nA -> a | λ\nB -> bB | λ');
const inputs = {};
for (const [name, text] of [['messy', messy], ['nullable', nullable]]) {
  const start = (await api({ op: 'normalize', document: text })).document;
  for (const to of ['remove_lambda', 'remove_unit', 'remove_useless', 'cnf']) {
    inputs[`${name}-${to}`] = { input: start, result: await api({ op: 'convert', document: start, to }) };
  }
}
save('grammar-transforms', inputs);

save('parse', {
  grammar: 'S -> aSb | λ',
  ll1: await api({ op: 'parse', grammar: 'S -> aSb | λ', input: 'aabb', parser: 'll1' }),
  ll1Rejected: await api({ op: 'parse', grammar: 'S -> aSb | λ', input: 'aab', parser: 'll1' }),
  slr1: await api({ op: 'parse', grammar: 'S -> aSb | λ', input: 'aabb', parser: 'slr1' }),
  brute: await api({ op: 'parse', grammar: 'S -> aSb | λ', inputs: ['aabb', 'aab', ''], parser: 'brute' }),
  cyk: await api({ op: 'parse', grammar: 'S -> AB\nA -> a\nB -> b', input: 'ab', parser: 'cyk' }),
  tables: await api({ op: 'parse_tables', grammar: expr }),
  conflicts: await api({ op: 'parse_tables', grammar: 'E -> E+E | i' }),
  user: await api({ op: 'user_parse', grammar: 'S -> aSb | λ', target: 'aabb', moves: [{ production: 0 }] }),
  userDone: await api({ op: 'user_parse', grammar: 'S -> aSb | λ', target: 'aabb', moves: [{ production: 0 }, { production: 0 }, { production: 1 }] }),
});

// automata conversions with steps
const gtgSource = (await api({ op: 'normalize', document: { type: 'fa', initial: 'q0', finals: ['q0', 'q1'], transitions: [['q0', 'a', 'q1'], ['q1', 'b', 'q0'], ['q1', 'c', 'q1'], ['q1', 'd', 'q1']] } })).document;
const nfaSource = (await api({ op: 'normalize', document: nfa })).document;
save('conversions', {
  nfaSource,
  dfa: await api({ op: 'convert', document: nfaSource, to: 'dfa' }),
  minimal: await api({ op: 'convert', document: nfaSource, to: 'minimal_dfa' }),
  gtgSource,
  toRegex: await api({ op: 'convert', document: gtgSource, to: 'regex' }),
  toRegexSimple: await api({ op: 'convert', document: nfaSource, to: 'regex' }),
  regexNfa: await api({ op: 'convert', document: { type: 'regex', expression: '(a+b)*abb' }, to: 'nfa' }),
});

save('design', {
  ok: await api({ op: 'design', spec: { alphabet: 'ab', language: { constraint_text: 'ends_with("ab")' }, examples: { accept: ['ab', 'aab'], reject: ['', 'ba'] } } }),
  failing: await api({ op: 'design', spec: { alphabet: 'ab', language: { constraint_text: 'ends_with("ab")' }, examples: { accept: ['ab', 'b'], reject: ['ba'] } } }),
  template: await api({ op: 'design', spec: { alphabet: 'ab', language: { template: { name: 'an_bn', a: 'a', b: 'b' } } } }),
  syntaxError: await api({ op: 'design', spec: { alphabet: 'ab', language: { constraint_text: 'ends_with("ab") and ' } } }).catch((e) => ({ error: { code: e.code, message: e.message, details: e.details } })),
});

const pumping = {};
const catalogue = await api({ op: 'pumping_languages' });
pumping.languages = { total: catalogue.languages.length, languages: catalogue.languages.filter((l) => l.index < 2) };
const play = async (kind, language, mode, moves) => {
  const first = await api({ op: 'pumping_new', kind, language, mode, seed: 1 });
  const out = [first];
  let doc = first.document;
  for (const move of moves) {
    const r = await api({ op: 'pumping_move', game: doc, move });
    out.push(r);
    doc = r.document;
  }
  return out;
};
pumping.regularComputerFirst = await play('regular', 0, 'computer_first', [{ move: 'choose_w', w: 'aaaabbbb' }, { move: 'choose_i', i: 2 }]);
pumping.regularHumanFirst = await play('regular', 0, 'human_first', [{ move: 'choose_m', m: 4 }]);
pumping.contextFreeHumanFirst = await play('context_free', 0, 'human_first', [{ move: 'choose_m', m: 3 }, { move: 'choose_decomposition', lengths: [0, 1, 1, 0] }, { move: 'add_case' }]);
save('pumping', pumping);

const lsystem = { type: 'lsystem', axiom: 'g', rules: ['g -> g + g - g - g + g'], parameters: { angle: '90', distance: '4' } };
save('lsystem', {
  normalized: (await api({ op: 'normalize', document: lsystem })).document,
  rendered: await api({ op: 'lsystem_render', document: lsystem, depth: 1 }),
});

// Turing machines and ALL in the notation of the course (CLASE 39): the palindrome MT and the ALL "IGUAL"
const palindrome = (await api({
  op: 'normalize',
  document: {
    type: 'turing',
    states: ['1', '2', '3', '4', '5', '6', '7', '8'],
    initial: '1',
    finals: ['8'],
    transitions: [
      ['1', '(a,Δ,D)', '2'], ['1', '(b,Δ,D)', '5'], ['1', '(Δ,Δ,D)', '8'],
      ['2', '(a,a,D) / (b,b,D)', '2'], ['2', '(Δ,Δ,I)', '3'],
      ['3', '(a,Δ,I)', '4'], ['3', '(Δ,Δ,D)', '8'],
      ['4', '(a,a,I) / (b,b,I)', '4'], ['4', '(Δ,Δ,D)', '1'],
      ['5', '(a,a,D) / (b,b,D)', '5'], ['5', '(Δ,Δ,I)', '6'],
      ['6', '(b,Δ,I)', '7'], ['6', '(Δ,Δ,D)', '8'],
      ['7', '(a,a,I) / (b,b,I)', '7'], ['7', '(Δ,Δ,D)', '1'],
    ],
  },
})).document;
const igual = (await api({
  op: 'normalize',
  document: {
    type: 'turing',
    states: ['I', 'A', 'B', 'C', 'D', 'F'],
    initial: 'I',
    finals: ['F'],
    transitions: [
      ['I', '(a,X,I)', 'A'], ['I', '(b,b,D) / (X,X,D)', 'I'], ['I', '(⟩,⟩,I)', 'D'],
      ['A', '(b,b,I) / (X,X,I)', 'A'], ['A', '(⟨,⟨,D)', 'B'],
      ['B', '(a,a,D)', 'B'], ['B', '(b,X,I)', 'C'], ['B', '(X,X,D)', 'B'],
      ['C', '(a,a,I) / (X,X,I)', 'C'], ['C', '(⟨,⟨,D)', 'I'],
      ['D', '(X,X,I)', 'D'], ['D', '(⟨,⟨,D)', 'F'],
    ],
  },
})).document;
const loop = (await api({ op: 'normalize', document: { type: 'turing', states: ['p', 'q'], initial: 'p', finals: ['q'], transitions: [['p', '(Δ,Δ,D)', 'p'], ['p', '(a,a,D)', 'p']] } })).document;
const broken = (await api({ op: 'normalize', document: { type: 'turing', states: ['p', 'q'], initial: 'p', finals: ['q'], transitions: [['p', '(a,a,D)', 'p'], ['p', '(⟩,⟩,D)', 'q'], ['p', '(⟨,x,I)', 'q']] } })).document;
const twoTapes = (await api({ op: 'normalize', document: { type: 'turing', states: ['p', 'q'], initial: 'p', finals: ['q'], transitions: [['p', 'a;b,R|c;d,R', 'q'], ['p', 'a;b,R|c;d,L', 'p']] } })).document;
const tkGrammar = 'S -> T\nDB -> BD\nD -> c\nT -> aTBD\nT -> abD\nbB -> bb';
save('catedra', {
  palindrome,
  igual,
  loop,
  broken,
  twoTapes,
  palindromeRuns: await api({ op: 'catedra_run', document: palindrome, inputs: ['aba', 'ab', 'bb', ''] }),
  igualRuns: await api({ op: 'catedra_run', document: igual, inputs: ['ab', 'b'], model: 'all' }),
  loopRun: await api({ op: 'catedra_run', document: loop, input: 'a', max_steps: 5 }),
  brokenRun: await api({ op: 'catedra_run', document: broken, input: 'a', model: 'all' }),
  palindromeDefinition: await api({ op: 'catedra_definition', document: palindrome }),
  igualDefinition: await api({ op: 'catedra_definition', document: igual, model: 'all' }),
  palindromeTable: await api({ op: 'catedra_table', document: palindrome }),
  igualCheck: await api({ op: 'all_check', document: igual, inputs: ['ab'] }),
  brokenCheck: await api({ op: 'all_check', document: broken, inputs: ['a'] }),
  analysisTwoTapes: await api({ op: 'analyze', document: twoTapes }),
  composed: await api({ op: 'compose_tm', a: palindrome, b: palindrome }),
  svg: (await api({ op: 'render', document: palindrome, format: 'svg' })).content,
  membership: { grammar: tkGrammar, report: await api({ op: 'membership_tk', grammar: tkGrammar, input: 'abbc' }) },
  membershipContracting: await api({ op: 'membership_tk', grammar: 'S -> aS | A\nA -> λ', input: 'ab' }).catch((e) => ({ error: { code: e.code, message: e.message, details: e.details } })),
});
