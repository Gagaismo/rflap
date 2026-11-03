// Pure replay models of the automata conversions: each one turns the `steps` list the engine returns into
// one frame per step (the state of the conversion after that step), so the views only have to draw them.

const GTG_LAMBDA = 'λ';

/** Which view fits a list of steps: 'subset', 'minimize', 'gtg' (FA to RE), 'regex-nfa', or null. */
export function conversionKind(steps) {
  if (!Array.isArray(steps) || !steps.length) return null;
  const first = steps[0];
  if (first.kind === 'new_state') return 'subset';
  if (first.kind === 'initial_partition') return 'minimize';
  if (['add_final_state', 'collapse_parallel', 'add_empty_transitions', 'eliminate_state'].includes(first.kind)) return 'gtg';
  if (['decat', 'destar', 'deparens', 'deor'].includes(first.action)) return 'regex-nfa';
  return null;
}

// ---------------------------------------------------------------- NFA -> DFA

/** After each step: the DFA states with the NFA states they stand for, the transitions so far, what to highlight. */
export function subsetFrames(steps) {
  const states = [];
  const transitions = [];
  return steps.map((step) => {
    let added = null;
    let focus = { states: [], nfa: [] };
    if (step.kind === 'new_state') {
      added = { state: step.state, nfa: step.nfa_states };
      states.push(added);
      focus = { states: [step.state], nfa: step.nfa_states };
    } else if (step.kind === 'add_transition') {
      transitions.push({ from: step.from, symbol: step.symbol, to: step.to });
      focus = { states: [...new Set([step.from, step.to])], nfa: [] };
    }
    return { step, states: states.map((s) => ({ ...s })), transitions: transitions.map((x) => ({ ...x })), added, focus };
  });
}

// ---------------------------------------------------------------- minimization

const groupKey = (group) => group.join(',');

/** The partition after each step, the groups the step produced, and the keys of every group seen so far (for the tree). */
export function partitionFrames(steps) {
  let groups = [];
  const seen = new Set();
  return steps.map((step) => {
    let changed = [];
    if (step.kind === 'initial_partition') {
      groups = step.groups.map((g) => [...g]);
      changed = groups;
    } else if (step.kind === 'split') {
      const at = groups.findIndex((g) => groupKey(g) === groupKey(step.group));
      if (at >= 0) groups = [...groups.slice(0, at), ...step.into.map((g) => [...g]), ...groups.slice(at + 1)];
      changed = step.into;
    }
    groups.forEach((g) => seen.add(groupKey(g)));
    return {
      step,
      groups: groups.map((g) => [...g]),
      changed: changed.map((g) => [...g]),
      symbol: step.kind === 'split' ? step.symbol : null,
      merge: step.kind === 'merge_group' ? step.state : null,
      seen: [...seen],
    };
  });
}

/**
 * The distinguishability tree as far as the replay has got: a node shows its children (and the symbol that
 * told them apart) only once all of them have appeared as groups of the partition.
 */
export function prunedTree(tree, seenKeys) {
  const seen = new Set(seenKeys);
  const walk = (node) => {
    const expanded = node.children.length > 0 && node.children.every((c) => seen.has(groupKey(c.states)));
    return { states: node.states, symbol: expanded ? node.symbol : null, children: expanded ? node.children.map(walk) : [] };
  };
  return walk(tree);
}

/** `{q0, q1}` for a node of the tree. */
export const treeNodeLabel = (node, nameOf) => `{${node.states.map(nameOf).join(', ')}}`;

// ---------------------------------------------------------------- FA -> RE: the generalized transition graph

const labelOf = (label) => (label === '' ? GTG_LAMBDA : label);
const GTG_EMPTY = 'ø';

function sortedIds(set) {
  return [...set].sort((a, b) => a - b);
}

/**
 * The graph after each step of the state elimination: `{step, states, finals, edges: [{from, to, label}],
 * eliminated, emptyPairs, expression}`. Edges that stand for ø are not drawn (`emptyPairs` counts them).
 * `source` is the full FA document the conversion started from.
 */
export function gtgFrames(source, steps) {
  const states = new Set(Object.keys(source.states).map(Number));
  let finals = [...source.finals];
  let edges = source.transitions.map((x) => ({ from: x.from, to: x.to, label: labelOf(x.label) }));
  return steps.map((step) => {
    let eliminated = null;
    let emptyPairs = 0;
    let expression = null;
    switch (step.kind) {
      case 'add_final_state':
        states.add(step.state);
        edges = [...edges, ...step.old_finals.map((from) => ({ from, to: step.state, label: GTG_LAMBDA }))];
        finals = [step.state];
        break;
      case 'collapse_parallel':
        edges = [...edges.filter((e) => !(e.from === step.from && e.to === step.to)), { from: step.from, to: step.to, label: step.label }];
        break;
      case 'add_empty_transitions':
        emptyPairs = step.pairs.length;
        break;
      case 'eliminate_state':
        states.delete(step.state);
        eliminated = step.state;
        edges = step.edges.filter((e) => e.label !== GTG_EMPTY).map((e) => ({ ...e }));
        break;
      case 'result':
        expression = step.expression;
        break;
      default:
        break;
    }
    return { step, states: sortedIds(states), finals: [...finals], edges: edges.map((e) => ({ ...e })), eliminated, emptyPairs, expression };
  });
}

const pointFor = (index) => ({ x: 60 + (index % 6) * 130, y: 60 + Math.floor(index / 6) * 110 });

/** A full FA document of a frame, with the positions of the source states, ready for the engine's `render`. */
export function gtgDocument(source, frame) {
  const states = {};
  frame.states.forEach((id, index) => {
    const known = source.states[id];
    states[id] = known ? { ...known } : { id, name: `q${id}`, label: null, output: null, point: pointFor(index) };
  });
  return {
    type: 'fa',
    initial: source.initial,
    finals: [...frame.finals],
    notes: [],
    states,
    transitions: frame.edges.filter((e) => e.label !== GTG_EMPTY).map((e) => ({ from: e.from, to: e.to, label: e.label === GTG_LAMBDA ? '' : e.label, control: null })),
  };
}

// ---------------------------------------------------------------- RE -> NFA (de-expressionify)

/** The generalized graph after each `decat` / `destar` / `deor` / `deparens` step, starting from one edge 0 → 1. */
export function regexNfaFrames(steps) {
  if (!steps.length) return [];
  const states = new Set([0, 1]);
  let edges = [{ from: 0, to: 1, label: steps[0].transition.label }];
  return steps.map((step) => {
    const gone = step.transition;
    const at = edges.findIndex((e) => e.from === gone.from && e.to === gone.to && e.label === gone.label);
    if (at >= 0) edges = edges.filter((_, i) => i !== at);
    edges = [...edges, ...step.added.map((e) => ({ ...e }))];
    step.new_states.forEach((id) => states.add(id));
    return { step, action: step.action, states: sortedIds(states), edges: edges.map((e) => ({ ...e })), replaced: { ...gone }, addedStates: [...step.new_states], addedEdges: step.added.map((e) => ({ ...e })) };
  });
}

/** A full FA document of a regex-to-NFA frame, positioned like the finished NFA (`finalDoc`). */
export function nfaGraphDocument(finalDoc, frame) {
  const states = {};
  for (const id of frame.states) states[id] = { ...finalDoc.states[id] };
  return {
    type: 'fa',
    initial: finalDoc.initial,
    finals: [...finalDoc.finals],
    notes: [],
    states,
    transitions: frame.edges.map((e) => ({ from: e.from, to: e.to, label: e.label === GTG_LAMBDA ? '' : e.label, control: null })),
  };
}
