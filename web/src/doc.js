// Helpers over the FULL document JSON (what the `normalize` op returns).
//
//   fa / pda / mealy / moore : { type, states: {id: {id,name,label,output,point:{x,y}}}, transitions: [{from,to,label,control}],
//                                initial: id|null, finals: [id], notes: [] }
//   turing                   : { type, tapes, automaton: <the object above without type>, blocks: {stateId: <inner machine>} }
//                              an inner machine is { automaton, tapes, blocks } (no type)
//
// Label shapes: fa/moore a string; pda {input,pop,push}; mealy {input,output}; turing {actions:[{read,write,direction}]}.
// All mutators work IN PLACE on the document they receive; callers clone first (`clone`) and commit the
// clone to the history. `path` is the list of state ids walked into building blocks (Turing machines).

export const AUTOMATA = ['fa', 'pda', 'turing', 'mealy', 'moore'];
export const HISTORY_LIMIT = 50; // JFLAP's default undo depth

export const isAutomaton = (type) => AUTOMATA.includes(type);
export const clone = (value) => JSON.parse(JSON.stringify(value));

// ---------------------------------------------------------------- machines and paths

/** The object holding `states`, `transitions`, `initial`, `finals` for the machine at `path`. */
export function container(doc, path = []) {
  let machine = doc;
  for (const id of path) {
    machine = machine.blocks && machine.blocks[id];
    if (!machine) throw new Error(`no building block at state ${id}`);
  }
  return doc.type === 'turing' || path.length ? machine.automaton : machine;
}

/** The machine at `path` as a standalone document, ready for `render`, `simulate`, ... */
export function machineDocument(doc, path = []) {
  if (!path.length) return doc;
  let machine = doc;
  for (const id of path) machine = machine.blocks[id];
  return { type: 'turing', ...machine };
}

/** The machine object at `path` itself (not a copy): the document, or the inner machine of a block. */
export function realMachine(doc, path = []) {
  let machine = doc;
  for (const id of path) machine = machine.blocks[id];
  return machine;
}

export function tapeCount(doc) {
  return doc.type === 'turing' ? doc.tapes || 1 : 1;
}

export const stateList = (doc, path = []) => Object.values(container(doc, path).states);
export const stateById = (doc, id, path = []) => container(doc, path).states[id];
export const stateByName = (doc, name, path = []) => stateList(doc, path).find((s) => s.name === name);
export const isFinal = (doc, id, path = []) => container(doc, path).finals.includes(id);

// ---------------------------------------------------------------- creation

/** A new empty document of an automaton type (no api needed; same shape as `normalize({type})`). */
export function emptyDocument(type, tapes = 1) {
  const body = { finals: [], initial: null, notes: [], states: {}, transitions: [] };
  if (type === 'turing') return { type, tapes, automaton: body, blocks: {} };
  return { type, ...body };
}

// ---------------------------------------------------------------- states

function freshName(c) {
  const used = new Set(Object.values(c.states).map((s) => s.name));
  let n = 0;
  for (const id of Object.keys(c.states)) n = Math.max(n, Number(id) + 1);
  while (used.has(`q${n}`)) n++;
  return `q${n}`;
}

/** Adds a state at (x, y) named `q<n>`; returns its id. */
export function addState(doc, x, y, path = []) {
  const c = container(doc, path);
  const id = Object.keys(c.states).reduce((m, k) => Math.max(m, Number(k) + 1), 0);
  c.states[id] = { id, label: null, name: freshName(c), output: doc.type === 'moore' ? '' : null, point: { x, y } };
  return id;
}

/** Removes a state with its transitions, its building block and its initial/final marks. */
export function removeState(doc, id, path = []) {
  const c = container(doc, path);
  delete c.states[id];
  c.transitions = c.transitions.filter((t) => t.from !== id && t.to !== id);
  c.finals = c.finals.filter((f) => f !== id);
  if (c.initial === id) c.initial = null;
  const owner = realMachine(doc, path);
  if (owner.blocks) delete owner.blocks[id];
}

/** Moves a state; explicit curve control points of its transitions follow by half the move per end. */
export function moveState(doc, id, x, y, path = []) {
  const c = container(doc, path);
  const s = c.states[id];
  const dx = x - s.point.x;
  const dy = y - s.point.y;
  s.point = { x, y };
  for (const t of c.transitions) {
    if (!t.control) continue;
    const share = (t.from === id ? 0.5 : 0) + (t.to === id ? 0.5 : 0);
    if (share) t.control = { x: t.control.x + dx * share, y: t.control.y + dy * share };
  }
}

export function setInitial(doc, id, path = []) {
  container(doc, path).initial = id;
}

export function toggleFinal(doc, id, path = []) {
  const c = container(doc, path);
  c.finals = c.finals.includes(id) ? c.finals.filter((f) => f !== id) : [...c.finals, id].sort((a, b) => a - b);
}

export function renameState(doc, id, name, path = []) {
  const c = container(doc, path);
  const clash = Object.values(c.states).some((s) => s.id !== id && s.name === name);
  if (!name.trim()) throw new Error('empty name');
  if (clash) throw new Error(`the name ${name} is already used`);
  c.states[id].name = name.trim();
}

export function setStateLabel(doc, id, label, path = []) {
  container(doc, path).states[id].label = label.trim() ? label : null;
}

export function setStateOutput(doc, id, output, path = []) {
  container(doc, path).states[id].output = output;
}

// ---------------------------------------------------------------- transitions

export function addTransition(doc, from, to, label, path = []) {
  const c = container(doc, path);
  c.transitions.push({ control: null, from, label, to });
  return c.transitions.length - 1;
}

export function removeTransition(doc, index, path = []) {
  container(doc, path).transitions.splice(index, 1);
}

export function setTransitionLabel(doc, index, label, path = []) {
  container(doc, path).transitions[index].label = label;
}

export function setTransitionControl(doc, index, point, path = []) {
  container(doc, path).transitions[index].control = point;
}

// ---------------------------------------------------------------- Turing building blocks

export function hasBlock(doc, id, path = []) {
  const owner = realMachine(doc, path);
  return Boolean(owner.blocks && owner.blocks[id]);
}

/** Turns a state into a building block with an empty inner machine (no-op if it already is one). */
export function ensureBlock(doc, id, path = []) {
  const machine = realMachine(doc, path);
  if (machine.blocks[id]) return;
  const { type, ...inner } = emptyDocument('turing', machine.tapes || 1);
  machine.blocks[id] = inner;
}

export function removeBlock(doc, id, path = []) {
  delete realMachine(doc, path).blocks[id];
}

// ---------------------------------------------------------------- label text

const DIRECTIONS = { Left: 'L', Right: 'R', Stay: 'S' };
const BLANK = '□';
const lambdaOr = (s) => (s === '' || s == null ? 'λ' : s);
const blankToUnderscore = (s) => (s === BLANK ? '_' : s);

/** The editable text of a label (the compact syntax of rflap-api, which `labelFromText` parses back). */
export function labelToText(type, label) {
  switch (type) {
    case 'pda':
      return `${lambdaOr(label.input)},${lambdaOr(label.pop)};${lambdaOr(label.push)}`;
    case 'mealy':
      return `${lambdaOr(label.input)};${lambdaOr(label.output)}`;
    case 'turing':
      return label.actions
        .map((a) => `${blankToUnderscore(a.read)};${blankToUnderscore(a.write)},${DIRECTIONS[a.direction] || a.direction}`)
        .join('|');
    default:
      return lambdaOr(label);
  }
}

/** A sensible starting text for the label form of a new transition. */
export function defaultLabelText(type, tapes = 1) {
  switch (type) {
    case 'pda':
      return 'a,Z;Z';
    case 'mealy':
      return 'a;0';
    case 'turing':
      return Array.from({ length: tapes }, () => 'a;a,R').join('|');
    default:
      return '';
  }
}

/** Parses label text with the engine (`normalize` of a one-transition compact machine); returns the full label. */
export async function labelFromText(api, type, text, tapes = 1) {
  const probe = { type, initial: 'a', transitions: [['a', text, 'a']] };
  if (type === 'turing') probe.tapes = tapes;
  const result = await api({ op: 'normalize', document: probe });
  const d = result.document;
  return (type === 'turing' ? d.automaton : d).transitions[0].label;
}

/** Any document form -> the full form. */
export async function normalizeDocument(api, document) {
  return (await api({ op: 'normalize', document })).document;
}

// ---------------------------------------------------------------- compact form (for chats and links)

/** The compact form of an automaton: short enough to put in a chat message. `points` keeps the layout. */
export function compactOf(doc, { points = false } = {}, path = []) {
  const c = container(doc, path);
  const machine = path.length ? machineDocument(doc, path) : doc;
  const name = (id) => c.states[id].name;
  const out = { type: doc.type };
  if (c.initial != null) out.initial = name(c.initial);
  if (c.finals.length) out.finals = c.finals.map(name);
  out.transitions = c.transitions.map((t) => [name(t.from), labelToText(doc.type, t.label), name(t.to)]);
  const used = new Set([...(c.initial != null ? [name(c.initial)] : []), ...c.finals.map(name), ...out.transitions.flatMap((t) => [t[0], t[2]])]);
  const all = stateList(doc, path).map((s) => s.name);
  if (all.some((n) => !used.has(n))) out.states = all;
  if (doc.type === 'turing') out.tapes = machine.tapes;
  if (doc.type === 'moore') {
    const outputs = {};
    for (const s of stateList(doc, path)) if (s.output) outputs[s.name] = s.output;
    if (Object.keys(outputs).length) out.outputs = outputs;
  }
  if (points) {
    out.points = {};
    for (const s of stateList(doc, path)) out.points[s.name] = [Math.round(s.point.x), Math.round(s.point.y)];
  }
  if (machine.blocks && Object.keys(machine.blocks).length) {
    out.blocks = {};
    for (const id of Object.keys(machine.blocks)) {
      out.blocks[name(Number(id))] = compactOf(doc, { points }, [...path, Number(id)]);
    }
  }
  return out;
}

/** A short plain-text description of an automaton for the chat. */
export function describe(doc) {
  const c = container(doc);
  const initial = c.initial != null ? c.states[c.initial].name : '—';
  const finals = c.finals.map((f) => c.states[f].name).join(', ') || '—';
  return `${doc.type}: ${Object.keys(c.states).length} states, ${c.transitions.length} transitions, initial ${initial}, final ${finals}`;
}

// ---------------------------------------------------------------- geometry of the canvas

/** Bounding box of the states and explicit control points: {minX,minY,maxX,maxY}; null for an empty machine. */
export function bounds(doc, path = []) {
  const c = container(doc, path);
  const points = [...Object.values(c.states).map((s) => s.point), ...c.transitions.filter((t) => t.control).map((t) => t.control)];
  if (!points.length) return null;
  return {
    minX: Math.min(...points.map((p) => p.x)),
    minY: Math.min(...points.map((p) => p.y)),
    maxX: Math.max(...points.map((p) => p.x)),
    maxY: Math.max(...points.map((p) => p.y)),
  };
}

// ---------------------------------------------------------------- undo / redo

/** Snapshot history (JFLAP keeps 50 undo levels). Snapshots are clones, so edits never alias them. */
export class History {
  constructor(limit = HISTORY_LIMIT) {
    this.limit = limit;
    this.past = [];
    this.future = [];
  }

  /** Call BEFORE changing the document, with the document as it was. */
  record(before) {
    this.past.push(clone(before));
    if (this.past.length > this.limit) this.past.shift();
    this.future = [];
  }

  get canUndo() {
    return this.past.length > 0;
  }

  get canRedo() {
    return this.future.length > 0;
  }

  /** Returns the previous document (or null), remembering `current` for redo. */
  undo(current) {
    if (!this.past.length) return null;
    this.future.push(clone(current));
    return this.past.pop();
  }

  redo(current) {
    if (!this.future.length) return null;
    this.past.push(clone(current));
    return this.future.pop();
  }
}
