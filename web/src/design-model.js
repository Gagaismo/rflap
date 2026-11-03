// Pure side of the design panel (S4): the form -> `design` request mapping with its validation messages,
// and the view model of the answer (states with meanings, regex, verification report).

import { t, registerStrings } from './i18n.js';

registerStrings({
  en: {
    'design.err.alphabet': 'Write the alphabet: the symbols the strings are made of, such as "ab".',
    'design.err.constraint': 'Write the constraint, for example: ends_with("ab") and count(a) % 2 == 0.',
    'design.err.regex': 'Write the regular expression, for example: (a+b)*abb.',
    'design.err.grammar': 'Write the regular grammar, one production per line, for example: S -> aS | bA.',
    'design.err.example_symbol': 'The example "{word}" uses a symbol that is not in the alphabet "{alphabet}".',
    'design.err.template_symbol': 'The symbol "{symbol}" of the template is not in the alphabet "{alphabet}".',
    'design.err.template_char': 'The template symbol {name} must be exactly one character.',
    'design.err.template_different': 'The two symbols of the template must be different.',
    'design.err.min': 'The minimum n is 0 or 1.',
    'design.err.pairs': 'Write the bracket pairs as two characters each, separated by spaces, for example: () [].',
    'design.err.check_len': 'The length checked by brute force must be a whole number from 0 to 12.',
  },
  es: {
    'design.err.alphabet': 'Escribí el alfabeto: los símbolos de las cadenas, por ejemplo "ab".',
    'design.err.constraint': 'Escribí la restricción, por ejemplo: ends_with("ab") and count(a) % 2 == 0.',
    'design.err.regex': 'Escribí la expresión regular, por ejemplo: (a+b)*abb.',
    'design.err.grammar': 'Escribí la gramática regular, una producción por línea, por ejemplo: S -> aS | bA.',
    'design.err.example_symbol': 'El ejemplo "{word}" usa un símbolo que no está en el alfabeto "{alphabet}".',
    'design.err.template_symbol': 'El símbolo "{symbol}" de la plantilla no está en el alfabeto "{alphabet}".',
    'design.err.template_char': 'El símbolo {name} de la plantilla debe ser exactamente un carácter.',
    'design.err.template_different': 'Los dos símbolos de la plantilla deben ser distintos.',
    'design.err.min': 'El mínimo n es 0 o 1.',
    'design.err.pairs': 'Escribí los pares de paréntesis con dos caracteres cada uno, separados por espacios, por ejemplo: () [].',
    'design.err.check_len': 'La longitud comprobada por fuerza bruta debe ser un entero de 0 a 12.',
  },
});

export const DESIGN_MODES = ['constraint_text', 'regex', 'regular_grammar', 'template'];
export const DESIGN_TEMPLATES = ['an_bn', 'an_bm', 'equal_count', 'balanced_brackets', 'wcwr', 'even_palindrome', 'palindrome'];
export const DESIGN_MAX_CHECK_LEN = 12;

export function designDefaults() {
  return {
    alphabet: 'ab',
    mode: 'constraint_text',
    text: '',
    template: 'an_bn',
    a: 'a',
    b: 'b',
    c: 'c',
    min: '0',
    relation: 'le',
    pairs: '()',
    accept: '',
    reject: '',
    checkLen: '8',
    trap: true,
  };
}

/** One example per line; `λ` or `""` is the empty string, blank lines are skipped. */
export function parseExampleLines(text) {
  return String(text || '')
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line !== '')
    .map((line) => (line === 'λ' || line === '""' ? '' : line));
}

const symbolsOf = (text) => [...new Set([...String(text || '')].filter((c) => !/[\s,]/u.test(c)))];

function templateOf(form, alphabet) {
  const known = new Set(alphabet);
  const symbol = (name, value) => {
    const chars = [...String(value || '').trim()];
    if (chars.length !== 1) throw new Error(t('design.err.template_char', { name }));
    if (!known.has(chars[0])) throw new Error(t('design.err.template_symbol', { symbol: chars[0], alphabet: alphabet.join('') }));
    return chars[0];
  };
  switch (form.template) {
    case 'an_bn':
    case 'an_bm': {
      const a = symbol('a', form.a);
      const b = symbol('b', form.b);
      if (a === b) throw new Error(t('design.err.template_different'));
      if (form.template === 'an_bm') return { name: 'an_bm', a, b, relation: form.relation };
      if (!['0', '1'].includes(String(form.min).trim())) throw new Error(t('design.err.min'));
      return { name: 'an_bn', a, b, min: Number(form.min) };
    }
    case 'equal_count': {
      const a = symbol('a', form.a);
      const b = symbol('b', form.b);
      if (a === b) throw new Error(t('design.err.template_different'));
      return { name: 'equal_count', a, b };
    }
    case 'balanced_brackets': {
      const tokens = String(form.pairs || '').trim().split(/\s+/).filter(Boolean);
      if (!tokens.length || tokens.some((p) => [...p].length !== 2)) throw new Error(t('design.err.pairs'));
      const pairs = tokens.map((p) => [...p]);
      for (const symbolOfPair of pairs.flat()) symbol('pair', symbolOfPair);
      return { name: 'balanced_brackets', pairs };
    }
    case 'wcwr':
      return { name: 'wcwr', c: symbol('c', form.c) };
    default:
      return { name: form.template };
  }
}

/**
 * The `design` request the form describes. Throws an `Error` whose message says what to fix (shown under the
 * form); nothing is sent to the engine until the form is consistent.
 */
export function designRequest(form) {
  const alphabet = symbolsOf(form.alphabet);
  if (!alphabet.length) throw new Error(t('design.err.alphabet'));
  const text = String(form.text || '').trim();
  let language;
  if (form.mode === 'constraint_text') {
    if (!text) throw new Error(t('design.err.constraint'));
    language = { constraint_text: text };
  } else if (form.mode === 'regex') {
    if (!text) throw new Error(t('design.err.regex'));
    language = { regex: text };
  } else if (form.mode === 'regular_grammar') {
    if (!text) throw new Error(t('design.err.grammar'));
    language = { regular_grammar: String(form.text).trim() };
  } else {
    language = { template: templateOf(form, alphabet) };
  }
  const known = new Set(alphabet);
  const examples = { accept: parseExampleLines(form.accept), reject: parseExampleLines(form.reject) };
  for (const word of [...examples.accept, ...examples.reject]) {
    if ([...word].some((c) => !known.has(c))) throw new Error(t('design.err.example_symbol', { word, alphabet: alphabet.join('') }));
  }
  const request = { op: 'design', spec: { alphabet: alphabet.join(''), language, examples } };
  const options = {};
  const length = String(form.checkLen ?? '8').trim();
  if (length !== '8') {
    const n = Number(length);
    if (length === '' || !Number.isInteger(n) || n < 0 || n > DESIGN_MAX_CHECK_LEN) throw new Error(t('design.err.check_len'));
    options.check_len = n;
  }
  if (form.trap === false) options.include_trap_state = false;
  if (Object.keys(options).length) request.options = options;
  return request;
}

/** The answer of `design` shaped for the report: states with meanings, regex, examples, brute-force check. */
export function designView(result) {
  const verification = result.verification || {};
  const examples = (verification.examples || []).map((e) => ({ word: e.word, expected: e.expected_accept, actual: e.actual, passed: e.passed, detail: e.detail }));
  const cross = verification.cross_check || {};
  return {
    kind: result.kind,
    ok: verification.ok === true,
    description: result.description,
    acceptance: result.acceptance,
    states: (result.states || []).map((s) => ({ name: s.name, meaning: s.meaning, initial: s.initial, accepting: s.accepting, accessWord: s.access_word })),
    regex: result.regex ?? null,
    stats: result.stats,
    accepted: result.accepted || [],
    examples,
    failed: examples.filter((e) => !e.passed),
    crossCheck: { ok: cross.ok !== false, checked: cross.strings_checked ?? 0, maxLen: cross.max_len ?? 0, mismatches: cross.mismatch_count ?? 0, inconclusive: cross.inconclusive ?? 0, regexAgrees: cross.regex_agrees ?? null },
  };
}

/** The text of a constraint split around the character a syntax error points at. */
export function constraintMarker(text, position) {
  const chars = [...text];
  const at = Math.max(0, Math.min(position, chars.length));
  return { before: chars.slice(0, at).join(''), at: chars[at] ?? '', after: chars.slice(at + 1).join('') };
}

/** A failed `design` call: its message and, for a constraint syntax error, where in the text it points. */
export function designErrorView(error, constraintText) {
  const details = error && error.details;
  const position = details && details.error === 'constraint_syntax' && Number.isInteger(details.position) ? details.position : null;
  return { message: error.message, position, marker: position === null ? null : constraintMarker(constraintText || '', position) };
}
