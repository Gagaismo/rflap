// The automaton canvas: the SVG drawn by the engine (`render` op) made interactive through the
// data attributes it carries (data-state-id, data-transition-index, data-from, data-to).
//
// JFLAP tools: select (attribute editor: select & drag states and curve control points), state creator,
// transition creator (drag from a state to a state), deleter; context menu on states; double-click on a
// transition edits its label; zoom; keyboard shortcuts. Every change is a clone of the document committed
// through `ctx.commit` (undo/redo live in the tab).

import { h, svgEl, clear } from './dom.js';
import { t } from './i18n.js';
import { toast, popupMenu, formDialog } from './dialogs.js';
import { askLabel } from './labels.js';
import {
  clone, container, machineDocument, realMachine, bounds, labelToText, stateById,
  addState, removeState, moveState, setInitial, toggleFinal, renameState, setStateLabel, setStateOutput,
  addTransition, removeTransition, setTransitionLabel, setTransitionControl, hasBlock, ensureBlock, removeBlock,
} from './doc.js';

export const TOOLS = ['select', 'state', 'transition', 'delete'];
const TOOL_KEYS = { 1: 'select', 2: 'state', 3: 'transition', 4: 'delete' };
const DRAG_THRESHOLD = 4; // pixels before a press becomes a drag
const MIN_ZOOM = 0.25;
const MAX_ZOOM = 4;

const idOf = (el) => {
  const node = el && el.closest && el.closest('[data-state-id]');
  return node ? Number(node.dataset.stateId) : null;
};
const indexOf = (el) => {
  const node = el && el.closest && el.closest('[data-transition-index]');
  return node ? Number(node.dataset.transitionIndex) : null;
};

export class Canvas {
  /** `ctx` is the tab context (see app.js): doc, path, api, commit, overlay(), flags, dark, enterBlock, ... */
  constructor(container_, ctx) {
    this.ctx = ctx;
    this.root = container_;
    this.tool = 'select';
    this.zoom = 1;
    this.seq = 0;
    this.draft = null;
    this.svg = null;
    this.selected = new Set();
    this.selectedTransition = null;

    this.stage = h('div', { class: 'stage', tabindex: '0', role: 'application', 'aria-label': t('canvas.label') });
    this.status = h('div', { class: 'canvas-status', role: 'status', 'aria-live': 'polite' });
    this.toolButtons = {};
    this.toolbar = this.buildToolbar();
    this.root.append(this.toolbar, this.stage, this.status);

    this.stage.addEventListener('pointerdown', (e) => this.onPointerDown(e));
    this.stage.addEventListener('dblclick', (e) => this.onDoubleClick(e));
    this.stage.addEventListener('contextmenu', (e) => this.onContextMenu(e));
    this.stage.addEventListener('keydown', (e) => this.onKeyDown(e));
    this.stage.addEventListener(
      'wheel',
      (e) => {
        if (!e.ctrlKey) return;
        e.preventDefault();
        this.setZoom(this.zoom * (e.deltaY < 0 ? 1.1 : 1 / 1.1));
      },
      { passive: false },
    );
    this.resizeObserver = typeof ResizeObserver === 'function' ? new ResizeObserver(() => this.layoutViewBox()) : null;
    if (this.resizeObserver) this.resizeObserver.observe(this.stage);
    this.setTool('select');
  }

  destroy() {
    if (this.resizeObserver) this.resizeObserver.disconnect();
    this.root.replaceChildren();
  }

  // ------------------------------------------------------------ toolbar

  buildToolbar() {
    const tool = (name, icon) => {
      const button = h('button', {
        type: 'button',
        class: 'tool',
        'aria-pressed': 'false',
        title: `${t(`tool.${name}`)} (${Object.keys(TOOL_KEYS).find((k) => TOOL_KEYS[k] === name)})`,
        'aria-label': t(`tool.${name}`),
        onclick: () => this.setTool(name),
      }, h('span', { 'aria-hidden': 'true' }, icon));
      this.toolButtons[name] = button;
      return button;
    };
    return h(
      'div',
      { class: 'toolbar', role: 'toolbar', 'aria-label': t('canvas.tools') },
      tool('select', '⬚'),
      tool('state', '◯'),
      tool('transition', '→'),
      tool('delete', '✕'),
      h('span', { class: 'sep' }),
      h('button', { type: 'button', class: 'tool', title: t('view.zoom_in'), 'aria-label': t('view.zoom_in'), onclick: () => this.setZoom(this.zoom * 1.25) }, '+'),
      h('button', { type: 'button', class: 'tool', title: t('view.zoom_out'), 'aria-label': t('view.zoom_out'), onclick: () => this.setZoom(this.zoom / 1.25) }, '−'),
      h('button', { type: 'button', class: 'tool', title: t('view.zoom_reset'), 'aria-label': t('view.zoom_reset'), onclick: () => this.setZoom(1) }, '1:1'),
    );
  }

  setTool(name) {
    this.tool = name;
    for (const [key, button] of Object.entries(this.toolButtons)) button.setAttribute('aria-pressed', String(key === name));
    this.stage.dataset.tool = name;
    this.announce(t(`tool.hint.${name}`));
  }

  setZoom(zoom) {
    this.zoom = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, zoom));
    this.layoutViewBox();
  }

  announce(message) {
    this.status.textContent = message;
  }

  // ------------------------------------------------------------ state of the view

  get doc() {
    return this.draft || this.ctx.doc;
  }

  get path() {
    return this.ctx.path;
  }

  get machine() {
    return machineDocument(this.doc, this.path);
  }

  get type() {
    return this.ctx.doc.type;
  }

  get tapes() {
    return realMachine(this.ctx.doc, this.path).tapes || 1;
  }

  select(states, transition = null) {
    this.selected = new Set(states);
    this.selectedTransition = transition;
    this.ctx.setSelection({ states: [...this.selected], transition });
    this.paintSelection();
  }

  selectAll() {
    this.select(Object.keys(container(this.ctx.doc, this.path).states).map(Number));
  }

  // ------------------------------------------------------------ drawing

  /** Re-renders from the (draft) document. Older in-flight renders are dropped. */
  async render() {
    const seq = ++this.seq;
    const ctx = this.ctx;
    const machine = this.machine;
    const overlay = ctx.overlay();
    const states = new Set(this.path.length ? [] : overlay.states);
    const transitions = new Set(this.path.length ? [] : overlay.transitions);
    try {
      if (ctx.flags.nondeterminism || ctx.flags.lambda) {
        const info = await ctx.api({ op: 'analyze', document: machine });
        if (ctx.flags.nondeterminism) {
          const byName = new Map(info.states.map((s) => [s.name, s.id]));
          for (const name of info.nondeterministic_states || []) states.add(byName.get(name));
        }
        if (ctx.flags.lambda) for (const l of info.lambda_transitions || []) transitions.add(l.index);
      }
      const result = await ctx.api({
        op: 'render',
        document: machine,
        format: 'svg',
        highlight_states: [...states].filter((s) => s !== undefined),
        highlight_transitions: [...transitions],
        dark: ctx.dark,
      });
      if (seq !== this.seq) return;
      this.show(result.content);
    } catch (error) {
      if (seq !== this.seq) return;
      clear(this.stage).append(h('p', { class: 'canvas-error', role: 'alert' }, error.message));
      this.svg = null;
    }
  }

  show(svgText) {
    const template = document.createElement('template');
    template.innerHTML = svgText;
    const svg = template.content.firstElementChild;
    const background = svg.querySelector('.background');
    if (background) background.remove();
    for (const edge of svg.querySelectorAll('g.transition')) {
      const curve = edge.querySelector('.curve');
      if (curve) {
        const hit = curve.cloneNode(false);
        hit.setAttribute('class', 'hit');
        hit.removeAttribute('stroke');
        edge.prepend(hit);
      }
    }
    svg.setAttribute('role', 'img');
    svg.setAttribute('aria-label', t('canvas.drawing', { text: this.describe() }));
    svg.removeAttribute('stroke');
    svg.classList.add('automaton');
    this.originalViewBox = svg.getAttribute('viewBox');
    this.svg = svg;
    this.stage.replaceChildren(svg);
    this.layoutViewBox();
    this.paintSelection();
    for (const id of this.blockStates()) {
      const node = svg.querySelector(`g.state[data-state-id="${id}"]`);
      if (node) node.classList.add('block');
    }
  }

  blockStates() {
    const machine = realMachine(this.ctx.doc, this.path);
    return Object.keys(machine.blocks || {}).map(Number);
  }

  describe() {
    const c = container(this.doc, this.path);
    const initial = c.initial != null && c.states[c.initial] ? c.states[c.initial].name : '—';
    return `${Object.keys(c.states).length} ${t('word.states')}, ${c.transitions.length} ${t('word.transitions')}, ${t('word.initial')} ${initial}`;
  }

  /** viewBox and pixel size: at least the visible stage, big enough for every state, scaled by the zoom. */
  layoutViewBox() {
    const svg = this.svg;
    if (!svg) return;
    const b = bounds(this.doc, this.path);
    const pad = 90;
    const x0 = b ? Math.min(0, b.minX - pad) : 0;
    const y0 = b ? Math.min(0, b.minY - pad) : 0;
    const needW = b ? b.maxX + pad - x0 : 0;
    const needH = b ? b.maxY + pad - y0 : 0;
    const w = Math.max(needW, (this.stage.clientWidth - 2) / this.zoom, 200);
    const h_ = Math.max(needH, (this.stage.clientHeight - 2) / this.zoom, 200);
    svg.setAttribute('viewBox', `${x0} ${y0} ${w} ${h_}`);
    svg.setAttribute('width', String(Math.ceil(w * this.zoom)));
    svg.setAttribute('height', String(Math.ceil(h_ * this.zoom)));
  }

  paintSelection() {
    if (!this.svg) return;
    for (const node of this.svg.querySelectorAll('g.state')) {
      node.classList.toggle('selected', this.selected.has(Number(node.dataset.stateId)));
    }
    for (const node of this.svg.querySelectorAll('g.transition')) {
      node.classList.toggle('selected', Number(node.dataset.transitionIndex) === this.selectedTransition);
    }
  }

  toCanvas(event) {
    const svg = this.svg;
    const point = svg.createSVGPoint();
    point.x = event.clientX;
    point.y = event.clientY;
    const mapped = point.matrixTransform(svg.getScreenCTM().inverse());
    return { x: Math.round(mapped.x), y: Math.round(mapped.y) };
  }

  // ------------------------------------------------------------ editing

  /** Applies `change` to a clone of the document and commits it. Returns false (and toasts) on error. */
  async edit(change, message) {
    const next = clone(this.ctx.doc);
    try {
      await change(next, this.path);
    } catch (error) {
      toast(error.message, 'error');
      return false;
    }
    await this.ctx.commit(next);
    if (message) this.announce(message);
    return true;
  }

  stateName(id) {
    const s = stateById(this.ctx.doc, id, this.path);
    return s ? s.name : String(id);
  }

  async createTransition(from, to) {
    const label = await askLabel({ api: this.ctx.api, type: this.type, tapes: this.tapes, title: t('label.new_transition', { from: this.stateName(from), to: this.stateName(to) }) });
    if (!label) return;
    await this.edit((d, path) => addTransition(d, from, to, label, path), t('canvas.transition_added', { from: this.stateName(from), to: this.stateName(to) }));
  }

  async editTransition(index) {
    const transition = container(this.ctx.doc, this.path).transitions[index];
    if (!transition) return;
    const label = await askLabel({
      api: this.ctx.api,
      type: this.type,
      tapes: this.tapes,
      text: labelToText(this.type, transition.label),
      title: t('label.edit_transition', { from: this.stateName(transition.from), to: this.stateName(transition.to) }),
    });
    if (label) await this.edit((d, path) => setTransitionLabel(d, index, label, path));
  }

  async renameDialog(id) {
    const state = stateById(this.ctx.doc, id, this.path);
    const result = await formDialog({
      title: t('state.rename'),
      fields: [{ name: 'name', label: t('state.name'), value: state.name }],
      validate: (v) => renameState(clone(this.ctx.doc), id, v.name, this.path),
    });
    if (result) await this.edit((d, path) => renameState(d, id, result.name, path));
  }

  async labelDialog(id) {
    const state = stateById(this.ctx.doc, id, this.path);
    const result = await formDialog({
      title: t('state.label'),
      fields: [{ name: 'label', label: t('state.label_text'), type: 'textarea', value: state.label || '' }],
    });
    if (result) await this.edit((d, path) => setStateLabel(d, id, result.label, path));
  }

  async outputDialog(id) {
    const state = stateById(this.ctx.doc, id, this.path);
    const result = await formDialog({
      title: t('state.output'),
      fields: [{ name: 'output', label: t('state.output_text'), value: state.output || '', lambda: true }],
    });
    if (result) await this.edit((d, path) => setStateOutput(d, id, result.output.trim() === 'λ' ? '' : result.output, path));
  }

  async removeSelection() {
    if (this.selectedTransition != null) {
      const index = this.selectedTransition;
      this.select([]);
      await this.edit((d, path) => removeTransition(d, index, path), t('canvas.transition_removed'));
    } else if (this.selected.size) {
      const ids = [...this.selected];
      this.select([]);
      await this.edit((d, path) => ids.forEach((id) => removeState(d, id, path)), t('canvas.state_removed'));
    }
  }

  // ------------------------------------------------------------ pointer

  onPointerDown(event) {
    if (event.button !== 0 || !this.svg) return;
    this.stage.focus({ preventScroll: true });
    const stateId = idOf(event.target);
    const index = stateId == null ? indexOf(event.target) : null;
    const point = this.toCanvas(event);
    switch (this.tool) {
      case 'select':
        return this.beginSelect(event, stateId, index, point);
      case 'state':
        if (stateId == null && index == null) {
          this.edit((d, path) => addState(d, point.x, point.y, path), t('canvas.state_added'));
        }
        return undefined;
      case 'transition':
        if (stateId != null) this.beginLink(event, stateId);
        return undefined;
      case 'delete':
        if (stateId != null) this.edit((d, path) => removeState(d, stateId, path), t('canvas.state_removed'));
        else if (index != null) this.edit((d, path) => removeTransition(d, index, path), t('canvas.transition_removed'));
        return undefined;
      default:
        return undefined;
    }
  }

  beginSelect(event, stateId, index, point) {
    if (stateId != null) {
      const keep = event.shiftKey || event.ctrlKey || event.metaKey;
      if (keep) {
        const next = new Set(this.selected);
        if (next.has(stateId)) next.delete(stateId);
        else next.add(stateId);
        this.select(next);
      } else if (!this.selected.has(stateId)) {
        this.select([stateId]);
      }
      const origins = new Map([...this.selected].map((id) => [id, { ...stateById(this.ctx.doc, id, this.path).point }]));
      this.startDrag(event, (e) => {
        const now = this.toCanvas(e);
        const draft = clone(this.ctx.doc);
        for (const [id, origin] of origins) moveState(draft, id, origin.x + now.x - point.x, origin.y + now.y - point.y, this.path);
        return draft;
      });
    } else if (index != null) {
      this.select([], index);
      const c = container(this.ctx.doc, this.path);
      const t_ = c.transitions[index];
      const a = c.states[t_.from].point;
      const b = c.states[t_.to].point;
      this.startDrag(event, (e) => {
        const now = this.toCanvas(e);
        const draft = clone(this.ctx.doc);
        // the curve passes through the pointer: control = 2·p − (a + b)/2
        setTransitionControl(draft, index, { x: 2 * now.x - (a.x + b.x) / 2, y: 2 * now.y - (a.y + b.y) / 2 }, this.path);
        return draft;
      });
    } else {
      this.select([]);
    }
  }

  /** Common drag loop: `draftAt(event)` builds the dragged document; it is committed on release if it moved. */
  startDrag(event, draftAt) {
    const start = { x: event.clientX, y: event.clientY };
    let moved = false;
    let frame = 0;
    const onMove = (e) => {
      if (!moved && Math.hypot(e.clientX - start.x, e.clientY - start.y) < DRAG_THRESHOLD) return;
      moved = true;
      this.draft = draftAt(e);
      if (!frame) {
        frame = requestAnimationFrame(() => {
          frame = 0;
          this.render();
        });
      }
    };
    const onUp = async () => {
      window.removeEventListener('pointermove', onMove);
      window.removeEventListener('pointerup', onUp);
      window.removeEventListener('pointercancel', onUp);
      const result = this.draft;
      this.draft = null;
      if (moved && result) await this.ctx.commit(result);
      else this.render();
    };
    window.addEventListener('pointermove', onMove);
    window.addEventListener('pointerup', onUp);
    window.addEventListener('pointercancel', onUp);
  }

  beginLink(event, from) {
    const start = stateById(this.ctx.doc, from, this.path).point;
    const line = svgEl('line', { class: 'rubber', x1: start.x, y1: start.y, x2: start.x, y2: start.y });
    this.svg.append(line);
    const onMove = (e) => {
      const p = this.toCanvas(e);
      line.setAttribute('x2', p.x);
      line.setAttribute('y2', p.y);
    };
    const onUp = (e) => {
      window.removeEventListener('pointermove', onMove);
      window.removeEventListener('pointerup', onUp);
      line.remove();
      const target = idOf(document.elementFromPoint(e.clientX, e.clientY));
      if (target != null) this.createTransition(from, target);
    };
    window.addEventListener('pointermove', onMove);
    window.addEventListener('pointerup', onUp);
  }

  onDoubleClick(event) {
    if (this.tool === 'transition' || this.tool === 'delete') return;
    const stateId = idOf(event.target);
    const index = stateId == null ? indexOf(event.target) : null;
    if (index != null) this.editTransition(index);
    else if (stateId != null) this.renameDialog(stateId);
  }

  // ------------------------------------------------------------ menus

  onContextMenu(event) {
    event.preventDefault();
    if (!this.svg) return;
    const stateId = idOf(event.target);
    const index = stateId == null ? indexOf(event.target) : null;
    if (stateId != null) {
      if (!this.selected.has(stateId)) this.select([stateId]);
      this.stateMenu(stateId, event.clientX, event.clientY);
    } else if (index != null) {
      this.select([], index);
      this.transitionMenu(index, event.clientX, event.clientY);
    } else {
      const p = this.toCanvas(event);
      popupMenu(event.clientX, event.clientY, [{ label: t('canvas.add_state_here'), onSelect: () => this.edit((d, path) => addState(d, p.x, p.y, path), t('canvas.state_added')) }]);
    }
  }

  stateMenu(id, x, y) {
    const c = container(this.ctx.doc, this.path);
    const path = this.path;
    const isBlock = hasBlock(this.ctx.doc, id, path);
    const items = [
      { label: t('state.initial'), checked: c.initial === id, onSelect: () => this.edit((d, p) => setInitial(d, c.initial === id ? null : id, p)) },
      { label: t('state.final'), checked: c.finals.includes(id), onSelect: () => this.edit((d, p) => toggleFinal(d, id, p)) },
      { separator: true },
      { label: t('state.rename'), onSelect: () => this.renameDialog(id) },
      { label: t('state.label'), onSelect: () => this.labelDialog(id) },
    ];
    if (this.type === 'moore') items.push({ label: t('state.output'), onSelect: () => this.outputDialog(id) });
    if (this.type === 'turing') {
      items.push({ separator: true });
      if (isBlock) {
        items.push({ label: t('block.enter'), onSelect: () => this.ctx.enterBlock(id) });
        items.push({ label: t('block.remove'), onSelect: () => this.edit((d, p) => removeBlock(d, id, p)) });
      } else {
        items.push({ label: t('block.make'), onSelect: () => this.edit((d, p) => ensureBlock(d, id, p)) });
      }
    }
    items.push({ separator: true }, { label: t('state.delete'), shortcut: 'Del', onSelect: () => this.edit((d, p) => removeState(d, id, p), t('canvas.state_removed')) });
    popupMenu(x, y, items, { label: this.stateName(id) });
  }

  transitionMenu(index, x, y) {
    popupMenu(x, y, [
      { label: t('transition.edit'), onSelect: () => this.editTransition(index) },
      { label: t('transition.straighten'), onSelect: () => this.edit((d, p) => setTransitionControl(d, index, null, p)) },
      { separator: true },
      { label: t('transition.delete'), shortcut: 'Del', onSelect: () => this.edit((d, p) => removeTransition(d, index, p), t('canvas.transition_removed')) },
    ]);
  }

  // ------------------------------------------------------------ keyboard

  onKeyDown(event) {
    if (event.ctrlKey || event.metaKey || event.altKey) return;
    const key = event.key;
    if (TOOL_KEYS[key]) {
      this.setTool(TOOL_KEYS[key]);
    } else if (key === 'Delete' || key === 'Backspace') {
      event.preventDefault();
      this.removeSelection();
    } else if (key === 'Escape') {
      this.select([]);
    } else if (key === 'F2' || (key === 'Enter' && this.selected.size === 1)) {
      if (this.selected.size === 1) this.renameDialog([...this.selected][0]);
      else if (this.selectedTransition != null) this.editTransition(this.selectedTransition);
    } else if (key === 'ContextMenu' || (key === 'F10' && event.shiftKey)) {
      event.preventDefault();
      const rect = this.stage.getBoundingClientRect();
      if (this.selected.size === 1) this.stateMenu([...this.selected][0], rect.left + 40, rect.top + 40);
    } else if (key.startsWith('Arrow') && this.selected.size) {
      event.preventDefault();
      const step = event.shiftKey ? 1 : 10;
      const dx = key === 'ArrowLeft' ? -step : key === 'ArrowRight' ? step : 0;
      const dy = key === 'ArrowUp' ? -step : key === 'ArrowDown' ? step : 0;
      this.edit((d, path) => {
        for (const id of this.selected) {
          const p = stateById(d, id, path).point;
          moveState(d, id, p.x + dx, p.y + dy, path);
        }
      });
    } else if (key === '+' || key === '=') {
      this.setZoom(this.zoom * 1.25);
    } else if (key === '-') {
      this.setZoom(this.zoom / 1.25);
    } else if (key === '0') {
      this.setZoom(1);
    }
  }
}
