// Pure helpers of the cátedra notation for Turing machines and linear bounded automata (ALL), shared by the
// label form, the panels and the tests. Nothing here touches the DOM or the engine.
//
//   notation  : 'catedra' -> (e,s,m) with Δ and I / N / D          'jflap' -> e;s,m with □ and L / S / R
//   model     : 'mt' (Turing machine) | 'all' (linear bounded automaton: the tape is ⟨w⟩)
//   directions: the FULL document keeps 'left' | 'stay' | 'right'; the compact text uses L / S / R (or I / N / D)

export const NOTATIONS = ['catedra', 'jflap'];
export const MODELS = ['mt', 'all'];
export const DELTA = 'Δ';
export const JFLAP_BLANK = '□';
export const LEFT_DELIMITER = '⟨';
export const RIGHT_DELIMITER = '⟩';
export const CONFIG_STEP = '├─';

export const notationOf = (value) => (NOTATIONS.includes(value) ? value : 'catedra');

/** The blank as the notation writes it. */
export const blankOf = (notation) => (notation === 'jflap' ? JFLAP_BLANK : DELTA);

// ---------------------------------------------------------------- model

const isDelimiter = (s) => typeof s === 'string' && (s.includes(LEFT_DELIMITER) || s.includes(RIGHT_DELIMITER));

const ASCII_LEFT = '<';
const ASCII_RIGHT = '>';

const actionsOf = (doc) => (doc && doc.type === 'turing' && doc.automaton ? doc.automaton.transitions.flatMap((t) => t.label.actions || []) : []);

/**
 * True when a Turing machine without ⟨ ⟩ uses both ASCII `<` and `>` as symbols: the pair is the alias of the
 * delimiters (a machine that uses only one keeps it as an ordinary symbol). Mirrors the rule of rflap-api.
 */
export function usesAsciiDelimiters(doc) {
  const actions = actionsOf(doc);
  if (actions.some((a) => isDelimiter(a.read) || isDelimiter(a.write))) return false;
  const has = (symbol) => actions.some((a) => a.read === symbol || a.write === symbol);
  return has(ASCII_LEFT) && has(ASCII_RIGHT);
}

/**
 * The document with `<` and `>` turned into ⟨ and ⟩ when `usesAsciiDelimiters` says so, else the same document.
 * The editor commits through it, so it never stores ASCII delimiters.
 */
export function convertAsciiDelimiters(doc) {
  if (!usesAsciiDelimiters(doc)) return doc;
  const swap = (s) => (s === ASCII_LEFT ? LEFT_DELIMITER : s === ASCII_RIGHT ? RIGHT_DELIMITER : s);
  const next = structuredClone(doc);
  for (const t of next.automaton.transitions) for (const a of t.label.actions || []) ((a.read = swap(a.read)), (a.write = swap(a.write)));
  return next;
}

/** 'all' when some transition reads or writes ⟨ or ⟩ (or `<` and `>` both, their alias), else 'mt'. */
export function inferModel(doc) {
  if (!doc || doc.type !== 'turing' || !doc.automaton) return 'mt';
  const used = actionsOf(doc).some((a) => isDelimiter(a.read) || isDelimiter(a.write));
  return used || usesAsciiDelimiters(doc) ? 'all' : 'mt';
}

export const modelOf = (value) => (MODELS.includes(value) ? value : 'mt');

// ---------------------------------------------------------------- label text

const DIRECTION_LETTER = { left: 'L', stay: 'S', right: 'R', L: 'L', S: 'S', R: 'R', I: 'L', N: 'S', D: 'R' };
const CATEDRA_MOVE = { L: 'I', S: 'N', R: 'D' };

/** The move letters offered by the form: I / N / D for a one-tape machine in the cátedra notation, else L / S / R. */
export function moveLetters(notation, tapes = 1) {
  return notation === 'catedra' && tapes === 1 ? ['I', 'N', 'D'] : ['L', 'S', 'R'];
}

/** A move letter or direction name -> 'L' | 'S' | 'R' (null if it is none of them). */
export function normalizeMove(move) {
  return DIRECTION_LETTER[String(move ?? '').trim()] || null;
}

/** 'L' | 'S' | 'R' -> the letter the notation writes. */
export function showMove(move, notation, tapes = 1) {
  const letter = normalizeMove(move) || 'S';
  return notation === 'catedra' && tapes === 1 ? CATEDRA_MOVE[letter] : letter;
}

const showSymbol = (s, notation) => {
  if (s === '' || s == null || s === JFLAP_BLANK || s === DELTA || s === '_') return blankOf(notation);
  return notation === 'catedra' ? s.replaceAll(JFLAP_BLANK, DELTA) : s.replaceAll(DELTA, JFLAP_BLANK);
};

/** The text of a full label `{actions: [{read, write, direction}]}` in the notation. */
export function labelText(label, notation = 'catedra') {
  const actions = label.actions || [];
  const tapes = actions.length;
  // a read pattern such as `a,b}v` has commas of its own: it only fits the JFLAP form
  const plain = actions.every((a) => !/[,;|]/.test(`${a.read}${a.write}`));
  return actions
    .map((a) => {
      const read = showSymbol(a.read, notation);
      const write = showSymbol(a.write, notation);
      if (notation === 'catedra' && tapes === 1 && plain) return `(${read},${write},${showMove(a.direction, notation, 1)})`;
      return `${read};${write},${showMove(a.direction, notation, plain ? tapes : 2)}`;
    })
    .join('|');
}

/**
 * Reads the text of one label in any of its forms: `(a,X,D)`, `a,X,D`, `a;X,R` and, for several tapes,
 * `a;X,R|b;Y,L`. Returns `[{read, write, move}]` with the blank as '' and the move as L / S / R, or null when
 * the text is not a label.
 */
export function parseLabelText(text) {
  const t = String(text ?? '').trim();
  if (!t) return null;
  const symbol = (s) => {
    const x = s.trim();
    return x === DELTA || x === JFLAP_BLANK || x === '_' ? '' : x;
  };
  const parts = [];
  if (t.includes(';')) {
    for (const part of t.split('|')) {
      const semi = part.indexOf(';');
      const comma = part.lastIndexOf(',');
      if (semi < 0 || comma < semi) return null;
      parts.push([part.slice(0, semi), part.slice(semi + 1, comma), part.slice(comma + 1)]);
    }
  } else {
    const inner = t.startsWith('(') && t.endsWith(')') ? t.slice(1, -1) : t;
    const pieces = inner.split(',');
    if (pieces.length !== 3) return null;
    parts.push(pieces);
  }
  const actions = parts.map(([read, write, move]) => ({ read: symbol(read), write: symbol(write), move: normalizeMove(move) }));
  return actions.every((a) => a.move) ? actions : null;
}

/** Label text -> the values of the form fields (`read0`, `write0`, `move0`, ...), shown in the notation. */
export function formValues(text, notation = 'catedra', tapes = 1) {
  const actions = parseLabelText(text) || [];
  const values = {};
  for (let i = 0; i < tapes; i++) {
    const a = actions[i] || { read: '', write: '', move: 'R' };
    values[`read${i}`] = a.read === '' ? '' : showSymbol(a.read, notation);
    values[`write${i}`] = a.write === '' ? '' : showSymbol(a.write, notation);
    values[`move${i}`] = showMove(a.move, notation, tapes);
  }
  return values;
}

/**
 * The values of the form fields -> the label text the engine parses. A non-empty `full` field (a typed
 * `(a,X,D)`) wins. An empty read or write field is the blank.
 */
export function textFromForm(values, notation = 'catedra', tapes = 1) {
  const full = String(values.full ?? '').trim();
  if (full) return full;
  const blank = blankOf(notation);
  const cell = (s) => {
    const x = String(s ?? '').trim();
    return x === '' ? blank : x;
  };
  const move = (i) => showMove(values[`move${i}`] || 'S', notation, tapes);
  if (notation === 'catedra' && tapes === 1) return `(${cell(values.read0)},${cell(values.write0)},${move(0)})`;
  return Array.from({ length: tapes }, (_, i) => `${cell(values[`read${i}`])};${cell(values[`write${i}`])},${move(i)}`).join('|');
}

// ---------------------------------------------------------------- configurations and runs

/**
 * The parts of one run of `catedra_run`: its configurations, how it ended and the class that colours it.
 * `tail` is what the engine appended to the sequence (`final/acepta`, `bloqueo/rechazo`, `no se detiene (...)`,
 * `violación del ALL: ...`).
 */
export function sequenceView(entry) {
  const configs = entry.configurations || [];
  const joined = configs.join(` ${CONFIG_STEP} `);
  const sequence = entry.sequence || joined;
  const tail = sequence.startsWith(joined) ? sequence.slice(joined.length).trim() : '';
  const verdictClass = { accepted: 'accept', rejected: 'reject', no_halt: 'nohalt', violation: 'violation' }[entry.verdict] || 'reject';
  return {
    input: entry.input,
    configs,
    tail,
    verdict: entry.verdict,
    verdictClass,
    accepted: Boolean(entry.accepted),
    steps: entry.steps ?? Math.max(0, configs.length - 1),
    violation: entry.violation || null,
    notes: entry.notes || [],
    text: sequence,
  };
}

/**
 * The state of a configuration written `c1...ci-1 qk ci...cn`. `names` are the state names; `previousTape` is the
 * tape (configuration without its state) of the previous step, used to choose between two readings when a tape
 * symbol looks like a state name. Returns `{name, tape}` or null.
 */
export function stateOfConfiguration(config, names, previousTape = null) {
  const candidates = [];
  for (const name of names) {
    for (let at = config.indexOf(name); at >= 0; at = config.indexOf(name, at + 1)) {
      candidates.push({ name, tape: config.slice(0, at) + config.slice(at + name.length) });
    }
  }
  if (!candidates.length) return null;
  if (candidates.length === 1 || previousTape == null) return candidates[0];
  const common = (a, b) => {
    let prefix = 0;
    while (prefix < a.length && prefix < b.length && a[prefix] === b[prefix]) prefix++;
    let suffix = 0;
    while (suffix < a.length - prefix && suffix < b.length - prefix && a[a.length - 1 - suffix] === b[b.length - 1 - suffix]) suffix++;
    return prefix + suffix;
  };
  return candidates.reduce((best, c) => (common(c.tape, previousTape) > common(best.tape, previousTape) ? c : best));
}

/** The state name of every configuration of a run, in order (the first one is the initial state when it fits). */
export function statesOfRun(configs, names, initialName = null) {
  const out = [];
  let previousTape = null;
  configs.forEach((config, i) => {
    let found = null;
    if (i === 0 && initialName && config.includes(initialName)) {
      found = { name: initialName, tape: config.replace(initialName, '') };
    } else {
      found = stateOfConfiguration(config, names, previousTape);
    }
    out.push(found ? found.name : null);
    previousTape = found ? found.tape : null;
  });
  return out;
}

// ---------------------------------------------------------------- definition, ALL check, Tk

/** `{a, b}` for a list, `∅` for none. */
export function formatSet(items) {
  return items && items.length ? `{${items.join(', ')}}` : '∅';
}

/** The formal definition as the course writes it: the header and one `{symbol, set}` row per component. */
export function definitionRows(definition) {
  const title = definition.model === 'all' ? 'ALL' : 'MT';
  const rows = [
    ['Q', formatSet(definition.states)],
    ['Σ_E', formatSet(definition.input_alphabet)],
    ['Σ_A', formatSet(definition.auxiliary_alphabet)],
    ['q0', definition.initial == null ? '—' : definition.initial],
    ['F', formatSet(definition.finals)],
  ];
  return { title, header: `${title} = ⟨Q, Σ_E, Σ_A, q0, F, f⟩`, rows: rows.map(([symbol, set]) => ({ symbol, set })) };
}

/** Indices of the transitions named by violations (the static ones and those of a run). */
export function violationTransitions(violations) {
  return [...new Set((violations || []).map((v) => v.transition).filter((x) => Number.isInteger(x)))].sort((a, b) => a - b);
}

/** Rows `T0 .. Tk` of a membership check; `same` is true when Tk repeats Tk-1 (the sets stopped growing). */
export function tkRows(report) {
  const sets = report.sets || [];
  return sets.map((forms, k) => {
    const same = k > 0 && sets[k - 1].length === forms.length && sets[k - 1].every((f, i) => f === forms[i]);
    return { name: `T${k}`, forms, same, text: same ? `T${k - 1}` : formatSet(forms.map((f) => (f === '' ? 'λ' : f))) };
  });
}

// ---------------------------------------------------------------- inputs of a run, multipista

/**
 * The inputs to run from the rows of a list: blank rows are skipped, `λ` is the empty word. With `fallback`
 * (a run) no row at all means the empty word; without it (the dynamic ALL check) it means no input.
 */
export function effectiveInputs(rows, { fallback = false } = {}) {
  const inputs = rows.map((r) => String(r ?? '').trim()).filter((r) => r !== '').map((r) => (r === 'λ' ? '' : r));
  return inputs.length || !fallback ? inputs : [''];
}

/** What `analyze` says about a multi-tape machine: null for one tape, else `{multipista, offending}`. */
export function multipistaOf(info) {
  const m = info && info.multipista;
  if (!m || !m.applicable) return null;
  return { multipista: Boolean(m.is_multipista), tapes: m.tapes, offending: m.offending || [] };
}

// ---------------------------------------------------------------- multiple run of a Turing machine

/** True when a typed input already carries the delimiters (`⟨aba⟩` or `<aba>`): the engine places its own. */
export function inputHasDelimiters(text) {
  const x = String(text ?? '').trim();
  return x.length >= 2 && (x.startsWith(LEFT_DELIMITER) || x.startsWith(ASCII_LEFT)) && (x.endsWith(RIGHT_DELIMITER) || x.endsWith(ASCII_RIGHT));
}

/** The criterion a Turing machine run accepts by: an ALL always by final state (the course), an MT as chosen. */
export function acceptanceUsed(model, chosen) {
  return model === 'all' ? 'final_state' : chosen || 'final_state';
}

/** The `catedra_run` request of a batch of inputs (the multiple run of an ALL: the tape is ⟨w⟩). */
export function catedraBatchRequest(document, inputs, model, maxSteps) {
  return { op: 'catedra_run', document, inputs, model, max_steps: maxSteps };
}

/** The runs of `catedra_run` as rows of the multiple run: `{outcome, accepted, notes, message}`. */
export function catedraRows(results) {
  return (results || []).map((r) => ({
    outcome: r.verdict === 'accepted' ? 'accepted' : r.verdict === 'no_halt' ? 'undecided' : 'rejected',
    accepted: Boolean(r.accepted),
    notes: r.notes || [],
    message: r.violation ? r.violation.message : null,
  }));
}
