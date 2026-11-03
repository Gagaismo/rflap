// A tab (a JFLAP window): one document with its undo history, view flags and overlays.
// The Tab IS the context handed to panels and actions (`ctx` in `mount(container, ctx)` / `run(ctx)`).

import { History, clone, isAutomaton } from './doc.js';

let nextTabId = 1;

export class Tab {
  /**
   * @param {object} app    the shell services: { api, host, t, dark, openTab(doc, options), notify(message, kind), tabs() }
   * @param {object} options { doc, title, steps, extras }
   */
  constructor(app, { doc, title, steps = null, extras = null }) {
    this.id = nextTabId++;
    this.app = app;
    this.doc = doc;
    this.title = title;
    this.path = []; // state ids walked into building blocks
    this.history = new History();
    this.flags = { nondeterminism: false, lambda: false };
    this.selection = { states: [], transition: null };
    this.steps = steps; // conversion steps: { kind, title, steps: [...] }
    this.extras = extras; // what the assistant sent: svg, analysis, verification, simulation, ...
    this.dirty = false;
    this.overlays = new Map();
    this.listeners = new Map();
  }

  // ---- services (shortcuts to the app)

  get api() {
    return this.app.api;
  }

  get host() {
    return this.app.host;
  }

  get t() {
    return this.app.t;
  }

  get dark() {
    return this.app.dark;
  }

  get type() {
    return this.doc.type;
  }

  get isAutomaton() {
    return isAutomaton(this.doc.type);
  }

  openTab(doc, options = {}) {
    return this.app.openTab(doc, options);
  }

  notify(message, kind = 'info') {
    this.app.notify(message, kind);
  }

  // ---- events: 'doc' | 'selection' | 'overlay' | 'path' | 'flags' | 'activate' | 'title'

  on(event, fn) {
    if (!this.listeners.has(event)) this.listeners.set(event, new Set());
    this.listeners.get(event).add(fn);
    return () => this.listeners.get(event).delete(fn);
  }

  emit(event, payload) {
    for (const fn of [...(this.listeners.get(event) || [])]) fn(payload);
  }

  // ---- the document

  /** Commits a new document (a clone edited by the caller): records undo history and notifies. */
  async commit(next) {
    this.history.record(this.doc);
    this.doc = next;
    this.dirty = true;
    this.emit('doc', { cause: 'edit' });
  }

  /** Applies `change(clone, path)` to a clone of the document and commits it; shows the error and returns false if it throws. */
  async edit(change) {
    const next = clone(this.doc);
    try {
      await change(next, this.path);
    } catch (error) {
      this.notify(error.message, 'error');
      return false;
    }
    await this.commit(next);
    return true;
  }

  /** Replaces the document without recording history (opening, converting in place, host updates). */
  replace(next, cause = 'replace') {
    this.doc = next;
    this.path = [];
    this.history = new History();
    this.overlays.clear();
    this.selection = { states: [], transition: null };
    this.emit('doc', { cause });
  }

  undo() {
    const previous = this.history.undo(this.doc);
    if (!previous) return false;
    this.doc = previous;
    this.dropMissingPath();
    this.emit('doc', { cause: 'undo' });
    return true;
  }

  redo() {
    const next = this.history.redo(this.doc);
    if (!next) return false;
    this.doc = next;
    this.dropMissingPath();
    this.emit('doc', { cause: 'redo' });
    return true;
  }

  dropMissingPath() {
    let machine = this.doc;
    const kept = [];
    for (const id of this.path) {
      if (!machine.blocks || !machine.blocks[id]) break;
      machine = machine.blocks[id];
      kept.push(id);
    }
    if (kept.length !== this.path.length) {
      this.path = kept;
      this.emit('path');
    }
  }

  snapshot() {
    return clone(this.doc);
  }

  setSteps(steps) {
    this.steps = steps;
    this.emit('steps');
  }

  setExtras(extras) {
    this.extras = extras;
    this.emit('extras');
  }

  // ---- view state

  setFlag(name, value) {
    this.flags[name] = value;
    this.emit('flags', name);
    this.emit('doc', { cause: 'flags' });
  }

  setSelection(selection) {
    this.selection = selection;
    this.emit('selection', selection);
  }

  /** Highlights from one source (`'sim'`, `'steps'`, ...) on the canvas; `null` clears the source. */
  setOverlay(source, overlay) {
    if (overlay) this.overlays.set(source, { states: overlay.states || [], transitions: overlay.transitions || [] });
    else this.overlays.delete(source);
    this.emit('overlay', source);
  }

  /** The union of every source: { states: Set<id>, transitions: Set<index> }. */
  overlay() {
    const states = new Set();
    const transitions = new Set();
    for (const o of this.overlays.values()) {
      o.states.forEach((s) => states.add(s));
      o.transitions.forEach((x) => transitions.add(x));
    }
    return { states, transitions };
  }

  enterBlock(id) {
    this.path = [...this.path, id];
    this.selection = { states: [], transition: null };
    this.emit('path');
    this.emit('doc', { cause: 'path' });
  }

  exitBlock() {
    this.path = this.path.slice(0, -1);
    this.selection = { states: [], transition: null };
    this.emit('path');
    this.emit('doc', { cause: 'path' });
  }
}
