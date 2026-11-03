// Pure helpers of the grammar environment (no DOM): the production table, the replay of the
// transformations, the parse tables and traces. Everything the engine computes stays in the engine; this
// only shapes its answers for display.

import { t, registerStrings } from './i18n.js';

registerStrings({
  en: {
    'grammar.err.no_arrow': 'Line {line}: write productions as "A -> aB | b" ("{text}" has no arrow).',
    'grammar.err.no_left': 'Line {line}: the production has no left side.',
    'grammar.step.DerivesLambda': '{variable} derives λ ({production}).',
    'grammar.step.RemoveLambda': 'Remove {production}.',
    'grammar.step.Expand': 'Replace {production} by: {added}.',
    'grammar.step.Edge': 'Unit production {production}: edge {from} → {to}.',
    'grammar.step.Reach': '{variable} reaches by unit productions: {list}.',
    'grammar.step.RemoveUnit': 'Remove the unit production {production}.',
    'grammar.step.Add': 'Add {production} (through {via}).',
    'grammar.step.DerivesTerminals': '{variable} derives a string of terminals ({production}).',
    'grammar.step.RemoveNonDeriving': 'Remove {production}: it uses a variable that derives no string.',
    'grammar.step.Reachable': '{variable} is reachable from the start variable.',
    'grammar.step.RemoveUnreachable': 'Remove {production}: its variable is unreachable.',
    'grammar.step.Terminals': 'Round {round}: replace the terminals of {production}; gives {added}.',
    'grammar.step.Split': 'Round {round}: split {production} into {added}.',
    'parse.action.replace': 'Replace {production}',
    'parse.action.match': 'Match {symbol}',
    'parse.action.accept': 'Accept',
    'parse.action.reject': 'Reject',
    'parse.action.reject_reason': 'Reject: {reason}',
    'parse.action.shift': 'Shift to state {state}',
    'parse.action.reduce': 'Reduce {production}',
    'parse.user.at': '{symbol} at {n}',
  },
  es: {
    'grammar.err.no_arrow': 'Línea {line}: escribí las producciones como "A -> aB | b" ("{text}" no tiene flecha).',
    'grammar.err.no_left': 'Línea {line}: la producción no tiene lado izquierdo.',
    'grammar.step.DerivesLambda': '{variable} deriva λ ({production}).',
    'grammar.step.RemoveLambda': 'Se quita {production}.',
    'grammar.step.Expand': 'Se reemplaza {production} por: {added}.',
    'grammar.step.Edge': 'Producción unitaria {production}: arista {from} → {to}.',
    'grammar.step.Reach': '{variable} alcanza por producciones unitarias: {list}.',
    'grammar.step.RemoveUnit': 'Se quita la producción unitaria {production}.',
    'grammar.step.Add': 'Se agrega {production} (a través de {via}).',
    'grammar.step.DerivesTerminals': '{variable} deriva una cadena de terminales ({production}).',
    'grammar.step.RemoveNonDeriving': 'Se quita {production}: usa una variable que no deriva ninguna cadena.',
    'grammar.step.Reachable': '{variable} es alcanzable desde la variable inicial.',
    'grammar.step.RemoveUnreachable': 'Se quita {production}: su variable es inalcanzable.',
    'grammar.step.Terminals': 'Ronda {round}: se reemplazan los terminales de {production}; queda {added}.',
    'grammar.step.Split': 'Ronda {round}: se divide {production} en {added}.',
    'parse.action.replace': 'Reemplazar {production}',
    'parse.action.match': 'Coincide {symbol}',
    'parse.action.accept': 'Aceptar',
    'parse.action.reject': 'Rechazar',
    'parse.action.reject_reason': 'Rechazar: {reason}',
    'parse.action.shift': 'Desplazar al estado {state}',
    'parse.action.reduce': 'Reducir {production}',
    'parse.user.at': '{symbol} en {n}',
  },
});

const GRAMMAR_LAMBDA = 'λ';

// ---------------------------------------------------------------- the production table

/** `A -> aB` with λ for the empty right side, as the grammar is written in the table and on the page. */
export const productionText = (p) => `${p.lhs} → ${p.rhs === '' ? GRAMMAR_LAMBDA : p.rhs}`;

const compact = (text) => text.replace(/\s+/g, '');
const emptyAlternative = (text) => text === '' || text === GRAMMAR_LAMBDA || text === 'ε';

/**
 * Reads `S -> aSb | λ` lines into table rows. Blank lines and `#` comments are skipped; `->` and `→` both
 * work; `λ`, `ε` or nothing is the empty string. Lines that cannot be read are returned as errors (1-based).
 */
export function parseProductionText(text) {
  const rows = [];
  const errors = [];
  String(text || '')
    .split(/\r?\n/)
    .forEach((raw, index) => {
      const line = raw.trim();
      if (line === '' || line.startsWith('#')) return;
      const arrow = /->|→/.exec(line);
      if (!arrow) {
        errors.push({ line: index + 1, message: t('grammar.err.no_arrow', { line: index + 1, text: line }) });
        return;
      }
      const lhs = compact(line.slice(0, arrow.index));
      if (lhs === '') {
        errors.push({ line: index + 1, message: t('grammar.err.no_left', { line: index + 1 }) });
        return;
      }
      for (const alternative of line.slice(arrow.index + arrow[0].length).split('|')) {
        const rhs = compact(alternative);
        rows.push({ lhs, rhs: emptyAlternative(rhs) ? '' : rhs });
      }
    });
  return { rows, errors };
}

/** The inverse of `parseProductionText`; consecutive productions of one variable share a line. */
export function serializeProductions(rows) {
  const lines = [];
  let last = null;
  for (const row of rows) {
    const rhs = row.rhs === '' ? GRAMMAR_LAMBDA : row.rhs;
    if (last !== null && lines.length && row.lhs === last) lines[lines.length - 1] += ` | ${rhs}`;
    else lines.push(`${row.lhs} -> ${rhs}`);
    last = row.lhs;
  }
  return lines.join('\n');
}

/** The start variable of a table: the chosen one if some production has it on the left, else the first left side. */
export function startVariable(rows, chosen) {
  if (chosen && rows.some((row) => row.lhs === chosen)) return chosen;
  const first = rows.find((row) => row.lhs !== '');
  return first ? first.lhs : '';
}

export const addGrammarRow = (rows, at = rows.length) => [...rows.slice(0, at), { lhs: '', rhs: '' }, ...rows.slice(at)];
export const removeGrammarRow = (rows, at) => rows.filter((_, i) => i !== at);
/** Edits a row; spaces are dropped and λ / ε (or nothing) on the right side is the empty string. */
export function setGrammarRow(rows, at, change) {
  const clean = {};
  if ('lhs' in change) clean.lhs = compact(change.lhs);
  if ('rhs' in change) clean.rhs = emptyAlternative(compact(change.rhs)) ? '' : compact(change.rhs);
  return rows.map((row, i) => (i === at ? { ...row, ...clean } : row));
}

/** Moves a row up (`-1`) or down (`1`); at the ends nothing moves. */
export function moveGrammarRow(rows, at, direction) {
  const to = at + direction;
  if (at < 0 || at >= rows.length || to < 0 || to >= rows.length) return rows;
  const copy = [...rows];
  [copy[at], copy[to]] = [copy[to], copy[at]];
  return copy;
}

// ---------------------------------------------------------------- transformations, step by step

const keyOf = (p) => `${p.lhs}\u0000${p.rhs}`;

/** A serde-tagged step (`{"Edge": {...}}`, `"Accept"`) as `{kind, ...fields}`; steps that carry `kind` already are kept. */
function flatStep(step) {
  if (step && typeof step === 'object' && 'kind' in step) return step;
  if (typeof step === 'string') return { kind: step };
  const [kind] = Object.keys(step || {});
  return kind === undefined ? { kind: '?' } : { kind, ...step[kind] };
}

const REMOVING = new Set(['RemoveLambda', 'RemoveUnit', 'RemoveNonDeriving', 'RemoveUnreachable']);

/** Applies one step to a list of productions: what it removes, what it adds, what it points at. */
function applyStep(list, step) {
  const removed = [];
  const added = [];
  let next = list;
  const take = (production) => {
    const at = next.findIndex((p) => keyOf(p) === keyOf(production));
    if (at >= 0) {
      removed.push(next[at]);
      next = next.filter((_, i) => i !== at);
    }
  };
  const put = (production) => {
    if (!next.some((p) => keyOf(p) === keyOf(production))) {
      next = [...next, production];
      added.push(production);
    }
  };
  if (REMOVING.has(step.kind)) take(step.production);
  else if (step.kind === 'Expand') {
    take(step.production);
    step.added.forEach(put);
  } else if (step.kind === 'Add') put(step.production);
  else if (step.kind === 'Terminals' || step.kind === 'Split') {
    take(step.production);
    step.replacement.forEach(put);
  }
  return { productions: next, removed, added };
}

/**
 * The stages of a transformation in JFLAP's order. A single `remove_*` is one stage; the Chomsky normal
 * form is lambda, unit, useless and the conversion proper. Each stage:
 * `{name, input: [productions], steps: [flat steps], result: [productions], renames?}`.
 */
export function grammarPhases(result, inputProductions) {
  const staged = result.steps && !Array.isArray(result.steps) ? result.steps : null;
  if (staged) {
    const phases = [];
    let input = inputProductions;
    for (const name of ['lambda', 'unit', 'useless', 'cnf']) {
      const phase = staged[name];
      if (!phase) continue;
      const out = phase.grammar.productions;
      phases.push({ name, input: name === 'cnf' ? phase.input.productions : input, steps: phase.steps.map(flatStep), result: out, renames: phase.renames || null });
      input = out;
    }
    return phases;
  }
  const names = { remove_lambda: 'lambda', remove_unit: 'unit', remove_useless: 'useless' };
  return [{ name: names[result.to] || result.to, input: inputProductions, steps: (result.steps || []).map(flatStep), result: result.document.productions, renames: null }];
}

/** Renames variables (`[[old, new], ...]`) everywhere at once; the old names are matched longest first. */
function renameProductions(list, renames) {
  if (!renames || !renames.length) return list;
  const to = new Map(renames);
  const escape = (name) => name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const pattern = new RegExp([...to.keys()].sort((a, b) => b.length - a.length).map(escape).join('|'), 'g');
  const swap = (text) => text.replace(pattern, (name) => to.get(name));
  return list.map((p) => ({ lhs: swap(p.lhs), rhs: swap(p.rhs) }));
}

/** The productions a stage reaches by applying its steps in order, and then its renaming of variables. */
export function replayedProductions(phase) {
  let list = phase.input;
  for (const step of phase.steps) list = applyStep(list, step).productions;
  return renameProductions(list, phase.renames);
}

/**
 * One frame per step plus a final frame per stage with the stage result:
 * `{phase, step: flat step | null, productions: [{lhs, rhs, mark: 'keep'|'added'|'focus'}], removed: [production]}`.
 */
export function grammarFrames(result, inputProductions) {
  const frames = [];
  for (const phase of grammarPhases(result, inputProductions)) {
    let list = phase.input;
    for (const step of phase.steps) {
      const outcome = applyStep(list, step);
      list = outcome.productions;
      const focus = step.production && !outcome.added.length ? keyOf(step.production) : null;
      frames.push({
        phase: phase.name,
        step,
        productions: list.map((p) => ({ ...p, mark: outcome.added.some((a) => keyOf(a) === keyOf(p)) ? 'added' : keyOf(p) === focus ? 'focus' : 'keep' })),
        removed: outcome.removed,
      });
    }
    frames.push({ phase: phase.name, step: null, productions: phase.result.map((p) => ({ ...p, mark: 'keep' })), removed: [], renames: phase.renames });
  }
  return frames;
}

/** A sentence in the user's language for a step ("A derives λ", "Remove S → A", ...). */
export function describeGrammarStep(step) {
  const flat = flatStep(step);
  const p = flat.production ? productionText(flat.production) : '';
  const known = `grammar.step.${flat.kind}`;
  if (t(known) === known) return flat.kind;
  const added = (flat.added || flat.replacement || []).map(productionText).join(', ');
  return t(known, { production: p, variable: flat.variable ?? '', from: flat.from ?? '', to: flat.to ?? '', via: flat.via ?? '', added, round: flat.round ?? '', list: (flat.reachable || []).join(', ') });
}

// ---------------------------------------------------------------- parse tables

const conflictCell = (entries) => entries.length > 1;

/** FIRST/FOLLOW rows, the LL(1) table and the SLR(1) items, ACTION and GOTO as plain rows, conflicts flagged. */
export function tablesModel(result) {
  const variables = Object.keys(result.first).sort();
  const ll = result.ll1;
  const lr = result.slr1;
  return {
    start: result.start,
    firstFollow: variables.map((variable) => ({ variable, first: result.first[variable], follow: result.follow[variable] || [] })),
    ll1: {
      ok: ll.is_ll1,
      terminals: ll.terminals,
      rows: ll.variables.map((variable) => ({
        variable,
        cells: ll.terminals.map((terminal) => {
          const entries = (ll.table[variable] && ll.table[variable][terminal]) || [];
          return { terminal, entries, conflict: conflictCell(entries) };
        }),
      })),
      conflicts: ll.conflicts,
    },
    slr1: {
      ok: lr.is_slr1,
      productions: lr.productions.map((text, n) => ({ n, text })),
      states: lr.states.map((s) => ({ id: s.id, items: s.items, final: s.is_final, transitions: s.transitions })),
      terminals: lr.terminals,
      variables: lr.variables,
      actionRows: lr.actions.map((row, state) => ({
        state,
        cells: lr.terminals.map((symbol) => {
          const entries = row[symbol] || [];
          return { symbol, entries, conflict: conflictCell(entries) };
        }),
      })),
      gotoRows: lr.gotos.map((row, state) => ({ state, cells: lr.variables.map((variable) => ({ variable, target: variable in row ? row[variable] : null })) })),
      conflicts: lr.conflicts,
    },
  };
}

// ---------------------------------------------------------------- parser traces

const TRACE_KINDS = { Replace: 'replace', Match: 'match', Accept: 'accept', Reject: 'reject', Shift: 'shift', Reduce: 'reduce' };

function actionParts(action) {
  if (typeof action === 'string') return [action, {}];
  const [kind] = Object.keys(action);
  return [kind, action[kind] || {}];
}

/** The action of a parser step in words ("Replace S → aSb", "Shift to state 2"). */
export function describeParseAction(action) {
  const [kind, body] = actionParts(action);
  switch (kind) {
    case 'Replace':
      return t('parse.action.replace', { production: productionText(body.production) });
    case 'Match':
      return t('parse.action.match', { symbol: body.symbol });
    case 'Accept':
      return t('parse.action.accept');
    case 'Reject':
      return body.reason ? t('parse.action.reject_reason', { reason: body.reason }) : t('parse.action.reject');
    case 'Shift':
      return t('parse.action.shift', { state: body.state });
    case 'Reduce':
      return t('parse.action.reduce', { production: productionText(body.production) });
    default:
      return kind;
  }
}

const kindOfAction = (action) => TRACE_KINDS[actionParts(action)[0]] || 'other';

/** Rows `{n, stack, input, action, kind}` of an LL(1) parse. */
export function ll1Trace(result) {
  return (result.steps || []).map((s, i) => ({ n: i + 1, stack: s.stack, input: s.input, action: describeParseAction(s.action), kind: kindOfAction(s.action) }));
}

/** Rows `{n, states, symbols, input, action, kind}` of an SLR(1) parse. */
export function slrTrace(result) {
  return (result.steps || []).map((s, i) => ({ n: i + 1, states: s.states.join(' '), symbols: s.symbols, input: s.input, action: describeParseAction(s.action), kind: kindOfAction(s.action) }));
}

/** The CYK triangle: row `i`, cells `j >= i` with the variables deriving the substring `i..j`. */
export function cykModel(table, input) {
  const chars = [...input];
  return {
    header: chars,
    rows: table.cells.map((row, i) => ({
      i,
      cells: row.map((symbols, j) => ({ j, symbols, substring: chars.slice(i, j + 1).join('') })).filter((cell) => cell.j >= i),
    })),
  };
}

/** The derivation as a list of `{n, production, form}`: the production applied and the form it gives. */
export function derivationSteps(result) {
  const derivation = result.derivation;
  if (!derivation) return [];
  return derivation.steps.map((s, i) => ({ n: i + 1, production: productionText(s.production), form: s.result }));
}

// ---------------------------------------------------------------- user-controlled parse

/** The move for a choice: one occurrence, or several at once (`positions`). */
export function userMove({ production, position, positions }) {
  if (positions && positions.length === 1) return { production, position: positions[0] };
  if (positions && positions.length > 1) return { production, positions };
  return position === undefined ? { production } : { production, position };
}

export const withoutLastMove = (moves) => moves.slice(0, -1);

/** What the user may still apply: `{production, text, positions, labels, all}` from the answer's `applicable`. */
export function userChoices(answer) {
  return (answer.applicable || []).map((a) => {
    const [lhs, ...rest] = a.text.split(' -> ');
    const rhs = rest.join(' -> ');
    return {
      production: a.production,
      text: productionText({ lhs, rhs: rhs === GRAMMAR_LAMBDA ? '' : rhs }),
      positions: a.positions,
      labels: a.positions.map((position) => t('parse.user.at', { symbol: lhs, n: position + 1 })),
      all: a.positions.length > 1,
    };
  });
}

/** The input lines of the text area: one string each; `λ` or an empty line is the empty string. */
export function parseInputLines(text) {
  const lines = String(text).split(/\r?\n/);
  while (lines.length > 1 && lines[lines.length - 1].trim() === '') lines.pop();
  return lines.map((line) => (line.trim() === 'λ' ? '' : line.trim()));
}
