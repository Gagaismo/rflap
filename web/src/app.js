// The app shell: menu bar, tabs (several documents open, like JFLAP windows), the canvas and the side panels.

import { h, clear, debounce } from './dom.js';
import { i18n, t, setLanguage } from './i18n.js';
import { Tab } from './tab.js';
import { Canvas } from './editor.js';
import { panelsFor, actionsFor, allActions, appliesTo } from './registry.js';
import { popupMenu, toast } from './dialogs.js';
import { emptyDocument, normalizeDocument, compactOf, describe } from './doc.js';
import { notationOf } from './tm-model.js';
import { contentExtras, resultNotation, resultTitle } from './tool-result.js';
import { documentFromHash } from './share.js';

const LANGUAGES = ['es', 'en'];
/** The UI is Spanish unless English was chosen explicitly (an old 'auto' setting means Spanish). */
const languageOf = (value) => (LANGUAGES.includes(value) ? value : 'es');

const MENUS = ['file', 'edit', 'input', 'test', 'convert', 'layout', 'view', 'help'];
const DOCUMENT_KEYS = ['document', 'automaton', 'machine'];

/** localStorage is not available in every sandboxed iframe: never let it throw. */
const store = {
  get(key) {
    try {
      return window.localStorage.getItem(key);
    } catch {
      return null;
    }
  },
  set(key, value) {
    try {
      window.localStorage.setItem(key, value);
    } catch {
      /* not persisted */
    }
  },
};

/** The document inside a tool result's `structuredContent`, and the rest as "extras". */
export function splitStructured(structured) {
  if (!structured || typeof structured !== 'object') return { document: null, extras: {} };
  let document_ = null;
  let used = null;
  for (const key of DOCUMENT_KEYS) {
    if (structured[key] && typeof structured[key] === 'object') {
      document_ = structured[key];
      used = key;
      break;
    }
  }
  if (!document_ && structured.result && structured.result.document) {
    document_ = structured.result.document;
    used = 'result';
  }
  if (!document_ && typeof structured.type === 'string' && (structured.transitions || structured.states)) {
    return { document: structured, extras: {} };
  }
  const extras = {};
  const source = used === 'result' ? { ...structured, ...structured.result } : structured;
  for (const [key, value] of Object.entries(source)) {
    if (key === used || key === 'result' || key === 'document' || value == null) continue;
    extras[key] = value;
  }
  return { document: document_, extras };
}

export class App {
  /**
   * @param {HTMLElement} root
   * @param {{api: Function, host: object}} services  `api` is the async `api(request)` of api.js
   */
  constructor(root, { api, host }) {
    this.root = root;
    this.api = api;
    this.host = host;
    this.t = t;
    this.tabList = [];
    this.active = null;
    this.workspaces = new Map(); // tab -> { el, canvas, panels: Map(id -> {container, instance}), activePanel }
    this.languageChoice = languageOf(store.get('rflap.language'));
    this.notation = notationOf(store.get('rflap.notation')); // how Turing machine labels are written
    this.hostContext = {};
    this.aiTab = null;
    this.viewerUrl = null;
    this.untitled = 0;
    this.syncModel = debounce((tab) => this.pushModelContext(tab), 700);
  }

  // ------------------------------------------------------------ theme and language

  /** Always the classic JFLAP look (white canvas, yellow states): neither the host theme nor prefers-color-scheme matter. */
  get dark() {
    return false;
  }

  applyTheme() {
    document.documentElement.dataset.theme = 'light';
  }

  /** Spanish or English, remembered in localStorage and (in ChatGPT) in the widget state. */
  setLanguage(choice) {
    this.languageChoice = languageOf(choice);
    store.set('rflap.language', this.languageChoice);
    this.applyLanguageChoice();
    if (this.mounted) this.rebuild();
    this.saveLanguageToHost();
  }

  /** Cátedra or JFLAP labels for Turing machines, remembered in localStorage like the language. */
  setNotation(choice) {
    this.notation = notationOf(choice);
    store.set('rflap.notation', this.notation);
    for (const tab of this.tabList) tab.emit('notation', tab.notation);
  }

  applyLanguageChoice() {
    setLanguage(this.languageChoice);
    document.documentElement.lang = i18n.locale;
  }

  saveLanguageToHost() {
    if (!this.host || this.host.mode !== 'chatgpt') return;
    if (this.active) {
      this.pushModelContext(this.active);
      return;
    }
    Promise.resolve(this.host.updateModelContext({ text: '', structured: { language: this.languageChoice } })).catch(() => {});
  }

  // ------------------------------------------------------------ boot and layout

  mount() {
    this.mounted = true;
    this.applyLanguageChoice();
    this.applyTheme();
    this.menubar = h('div', { class: 'menubar', role: 'menubar', 'aria-label': t('app.menus') });
    this.actionBar = h('div', { class: 'actionbar' });
    this.tabBar = h('div', { class: 'tabs', role: 'tablist', 'aria-label': t('app.tabs') });
    this.main = h('main', { class: 'main' });
    this.root.append(h('header', { class: 'header' }, h('span', { class: 'brand' }, 'Rflap'), this.menubar, this.actionBar), this.tabBar, this.main);
    document.addEventListener('keydown', (e) => this.onKey(e));
    window.addEventListener('dragover', (e) => e.preventDefault());
    window.addEventListener('drop', (e) => {
      e.preventDefault();
      if (e.dataTransfer && e.dataTransfer.files.length) this.openFiles([...e.dataTransfer.files]);
    });
    window.addEventListener('hashchange', () => this.openFromHash());
    this.renderChrome();
  }

  /** Rebuilds menus, tabs and every workspace (after a language change). */
  rebuild() {
    for (const tab of this.tabList) this.destroyWorkspace(tab);
    clear(this.menubar);
    clear(this.tabBar);
    clear(this.main);
    this.menubar.setAttribute('aria-label', t('app.menus'));
    this.tabBar.setAttribute('aria-label', t('app.tabs'));
    for (const tab of this.tabList) this.buildWorkspace(tab);
    this.renderChrome();
    if (this.active) this.activate(this.active);
  }

  renderChrome() {
    this.renderMenus();
    this.renderTabs();
    this.renderActionBar();
  }

  tabs() {
    return [...this.tabList];
  }

  notify(message, kind = 'info') {
    toast(message, kind);
  }

  pageUrl() {
    return this.viewerUrl || location.href;
  }

  // ------------------------------------------------------------ tabs

  /** Opens a new tab with `doc` (any document form is normalized to the full form). */
  openTab(doc, { title, steps = null, extras = null, activate = true } = {}) {
    const tab = new Tab(this, { doc, title: title || this.nextTitle(doc.type), steps, extras });
    this.tabList.push(tab);
    this.buildWorkspace(tab);
    tab.on('doc', (e) => {
      if (this.active === tab) this.renderActionBar();
      if (e.cause === 'edit' || e.cause === 'undo' || e.cause === 'redo') this.syncModel(tab);
    });
    tab.on('path', () => this.refreshWorkspace(tab));
    tab.on('caption', () => this.refreshWorkspace(tab));
    tab.on('steps', () => this.refreshPanels(tab));
    tab.on('extras', () => this.refreshPanels(tab));
    if (activate) this.activate(tab);
    else this.renderTabs();
    return tab;
  }

  newTab(doc, { model } = {}) {
    const tab = this.openTab(doc);
    if (model) tab.setModel(model);
    return tab;
  }

  nextTitle(type) {
    this.untitled += 1;
    return `${t(`type.${type}`)} ${this.untitled}`;
  }

  closeTab(tab) {
    const index = this.tabList.indexOf(tab);
    if (index < 0) return;
    this.destroyWorkspace(tab);
    this.tabList.splice(index, 1);
    if (this.aiTab === tab) this.aiTab = null;
    if (!this.tabList.length) this.openTab(emptyDocument('fa'));
    else if (this.active === tab) this.activate(this.tabList[Math.min(index, this.tabList.length - 1)]);
    else this.renderChrome();
  }

  activate(tab) {
    this.active = tab;
    for (const [other, ws] of this.workspaces) ws.el.hidden = other !== tab;
    this.renderChrome();
    tab.emit('activate');
    const ws = this.workspaces.get(tab);
    if (ws) {
      if (tab.canvas) tab.canvas.resized();
      this.showPanel(tab, ws.activePanel || (panelsFor(tab.type, tab)[0] || {}).id, undefined, { focus: false });
    }
  }

  renderTabs() {
    clear(this.tabBar);
    this.tabList.forEach((tab, index) => {
      const selected = tab === this.active;
      this.tabBar.append(
        h(
          'div',
          { class: `tab${selected ? ' active' : ''}` },
          h('button', {
            type: 'button',
            role: 'tab',
            id: `tab-${tab.id}`,
            'aria-selected': String(selected),
            tabindex: selected ? '0' : '-1',
            class: 'tab-title',
            onclick: () => this.activate(tab),
            onkeydown: (e) => {
              const delta = e.key === 'ArrowRight' ? 1 : e.key === 'ArrowLeft' ? -1 : 0;
              if (delta) {
                const next = this.tabList[(index + delta + this.tabList.length) % this.tabList.length];
                this.activate(next);
                const button = this.tabBar.querySelector(`#tab-${next.id}`);
                if (button) button.focus();
              }
            },
            text: `${tab.title}${tab.dirty ? ' •' : ''}`,
          }),
          h('button', { type: 'button', class: 'tab-close', 'aria-label': t('tab.close', { title: tab.title }), title: t('file.close'), onclick: () => this.closeTab(tab) }, '×'),
        ),
      );
    });
    this.tabBar.append(h('button', { type: 'button', class: 'tab-new', 'aria-label': t('file.new_fa'), title: t('file.new_fa'), onclick: () => this.openTab(emptyDocument('fa')) }, '+'));
  }

  renderActionBar() {
    clear(this.actionBar);
    const tab = this.active;
    if (!tab) return;
    const button = (label, onclick, disabled, title) => h('button', { type: 'button', class: 'icon', onclick, disabled, 'aria-label': title || label, title: title || label }, label);
    this.actionBar.append(button('↶', () => tab.undo(), !tab.history.canUndo, `${t('edit.undo')} (Ctrl+Z)`), button('↷', () => tab.redo(), !tab.history.canRedo, `${t('edit.redo')} (Ctrl+Y)`));
    if (this.host.connected && tab.isAutomaton) {
      this.actionBar.append(h('button', { type: 'button', class: 'primary', onclick: () => this.sendToChat(tab), title: t('chat.send_hint') }, t('chat.send')));
    }
  }

  // ------------------------------------------------------------ menus and shortcuts

  renderMenus() {
    clear(this.menubar);
    const tab = this.active;
    // menus registered by other modules (T12b) go before Help
    const extra = [...new Set(allActions().map((a) => a.menu))].filter((m) => !MENUS.includes(m));
    const order = [...MENUS.slice(0, -1), ...extra, MENUS[MENUS.length - 1]];
    for (const menu of order) {
      const items = tab ? actionsFor(menu, tab.type) : [];
      if (!items.length) continue;
      const button = h('button', {
        type: 'button',
        class: 'menu-button',
        role: 'menuitem',
        'aria-haspopup': 'menu',
        onclick: (e) => this.openMenu(menu, e.currentTarget),
        onkeydown: (e) => {
          if (e.key === 'ArrowDown' || e.key === 'Enter' || e.key === ' ') {
            e.preventDefault();
            this.openMenu(menu, e.currentTarget);
          } else if (e.key === 'ArrowRight' || e.key === 'ArrowLeft') {
            const all = [...this.menubar.children];
            const next = all[(all.indexOf(e.currentTarget) + (e.key === 'ArrowRight' ? 1 : -1) + all.length) % all.length];
            next.focus();
          }
        },
      }, this.menuLabel(menu));
      this.menubar.append(button);
    }
  }

  menuLabel(menu) {
    const text = t(`menu.${menu}`);
    return text === `menu.${menu}` ? menu : text;
  }

  openMenu(menu, anchor) {
    const tab = this.active;
    if (!tab) return;
    const rect = anchor.getBoundingClientRect();
    const items = [];
    for (const action of actionsFor(menu, tab.type)) {
      if (action.separator && items.length) items.push({ separator: true });
      items.push({
        label: t(action.label),
        shortcut: action.shortcut,
        disabled: action.enabled ? !action.enabled(tab) : false,
        checked: action.checked ? action.checked(tab) : undefined,
        onSelect: () => action.run(tab),
      });
    }
    popupMenu(rect.left, rect.bottom, items, { label: this.menuLabel(menu) });
  }

  onKey(event) {
    const tab = this.active;
    if (!tab) return;
    const target = event.target;
    const typing = target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.tagName === 'SELECT' || target.isContentEditable);
    const mod = event.ctrlKey || event.metaKey;
    if (!mod) return;
    const key = event.key.length === 1 ? event.key.toUpperCase() : event.key;
    if (key === 'Z' && event.shiftKey) {
      if (!typing) {
        event.preventDefault();
        tab.redo();
      }
      return;
    }
    for (const action of allActions()) {
      if (!action.shortcut || !appliesTo(action, tab.type)) continue;
      const parts = action.shortcut.split('+');
      const wanted = parts[parts.length - 1].toUpperCase();
      const wantShift = parts.includes('Shift');
      const matches = (wanted === key || (wanted === '=' && (key === '+' || key === '='))) && wantShift === event.shiftKey;
      if (!matches) continue;
      if (typing && !['Ctrl+S', 'Ctrl+O'].includes(action.shortcut)) return;
      if (action.enabled && !action.enabled(tab)) return;
      event.preventDefault();
      action.run(tab);
      return;
    }
  }

  // ------------------------------------------------------------ workspace of a tab

  buildWorkspace(tab) {
    const el = h('section', { class: 'workspace', 'aria-label': tab.title, hidden: true });
    const canvasArea = h('div', { class: 'canvas-area' });
    const breadcrumb = h('nav', { class: 'breadcrumb', 'aria-label': t('block.path') });
    const side = h('aside', { class: 'side' });
    const panelTabs = h('div', { class: 'panel-tabs', role: 'tablist', 'aria-label': t('app.panels') });
    const panelHost = h('div', { class: 'panel-host' });
    side.append(panelTabs, panelHost);
    const caption = h('div', { class: 'diagram-title', role: 'heading', 'aria-level': '2', hidden: true });
    el.append(h('div', { class: 'canvas-column' }, breadcrumb, caption, canvasArea), side);
    el.classList.toggle('graphless', !tab.isAutomaton);
    this.main.append(el);
    const ws = { el, canvasArea, breadcrumb, caption, panelTabs, panelHost, panels: new Map(), activePanel: null, canvas: null, offs: [] };
    this.workspaces.set(tab, ws);

    if (tab.isAutomaton) {
      ws.canvas = new Canvas(canvasArea, tab);
      tab.canvas = ws.canvas;
      const redraw = () => ws.canvas.render();
      // a document that arrives from outside (tool result, file, link, conversion) is fitted when drawn
      const loaded = (e) => {
        if (e && ['host', 'replace', 'open'].includes(e.cause)) ws.canvas.requestFit();
        redraw();
      };
      ws.offs.push(tab.on('doc', loaded), tab.on('overlay', redraw), tab.on('notation', redraw));
      redraw();
    }
    this.refreshWorkspace(tab);
  }

  destroyWorkspace(tab) {
    const ws = this.workspaces.get(tab);
    if (!ws) return;
    ws.offs.forEach((off) => off());
    for (const { instance } of ws.panels.values()) if (instance && instance.destroy) instance.destroy();
    if (ws.canvas) ws.canvas.destroy();
    if (tab.canvas === ws.canvas) tab.canvas = null;
    ws.el.remove();
    this.workspaces.delete(tab);
  }

  refreshWorkspace(tab) {
    const ws = this.workspaces.get(tab);
    if (!ws) return;
    clear(ws.breadcrumb);
    ws.breadcrumb.hidden = tab.path.length === 0;
    ws.caption.textContent = tab.caption || '';
    ws.caption.hidden = !tab.caption;
    if (tab.path.length) {
      let machine = tab.doc;
      const names = [tab.title];
      for (const id of tab.path) {
        const automaton = machine.automaton || machine;
        names.push((automaton.states[id] && automaton.states[id].name) || String(id));
        machine = machine.blocks[id];
      }
      ws.breadcrumb.append(h('button', { type: 'button', onclick: () => tab.exitBlock() }, `↰ ${t('block.exit')}`), h('span', {}, names.join(' ▸ ')));
    }
    this.refreshPanels(tab);
  }

  refreshPanels(tab) {
    const ws = this.workspaces.get(tab);
    if (!ws) return;
    const list = panelsFor(tab.type, tab);
    clear(ws.panelTabs);
    if (!list.some((p) => p.id === ws.activePanel)) ws.activePanel = list.length ? list[0].id : null;
    list.forEach((panel, index) => {
      const selected = panel.id === ws.activePanel;
      ws.panelTabs.append(
        h('button', {
          type: 'button',
          role: 'tab',
          class: selected ? 'panel-tab active' : 'panel-tab',
          'aria-selected': String(selected),
          tabindex: selected ? '0' : '-1',
          onclick: () => this.showPanel(tab, panel.id),
          onkeydown: (e) => {
            const delta = e.key === 'ArrowRight' ? 1 : e.key === 'ArrowLeft' ? -1 : 0;
            if (!delta) return;
            const next = list[(index + delta + list.length) % list.length];
            this.showPanel(tab, next.id);
            const buttons = [...ws.panelTabs.children];
            buttons[list.indexOf(next)].focus();
          },
          text: t(panel.title),
        }),
      );
    });
    if (ws.activePanel) this.showPanel(tab, ws.activePanel, undefined, { focus: false });
  }

  /**
   * Shows (mounting it the first time) a panel of a tab; returns the result of `method` on the panel's
   * instance when one is named (for example `'step'` on the simulate panel).
   */
  showPanel(tab, id, method, { focus = true } = {}) {
    const ws = this.workspaces.get(tab);
    if (!ws || !id) return undefined;
    const def = panelsFor(tab.type, tab).find((p) => p.id === id);
    if (!def) return undefined;
    ws.activePanel = id;
    let entry = ws.panels.get(id);
    if (!entry) {
      const container_ = h('div', { class: 'panel', role: 'tabpanel', 'aria-label': t(def.title) });
      ws.panelHost.append(container_);
      entry = { container: container_, instance: null };
      ws.panels.set(id, entry);
      try {
        entry.instance = def.mount(container_, tab) || null;
      } catch (error) {
        container_.append(h('p', { class: 'form-error', role: 'alert' }, error.message));
      }
    }
    for (const [panelId, e] of ws.panels) e.container.hidden = panelId !== id;
    for (const [index, button] of [...ws.panelTabs.children].entries()) {
      const selected = panelsFor(tab.type, tab)[index]?.id === id;
      button.classList.toggle('active', selected);
      button.setAttribute('aria-selected', String(selected));
      button.tabIndex = selected ? 0 : -1;
    }
    if (method && entry.instance && typeof entry.instance[method] === 'function') return entry.instance[method]();
    if (focus && entry.instance && entry.instance.focus) entry.instance.focus();
    return undefined;
  }

  // ------------------------------------------------------------ files

  pickFiles() {
    const input = h('input', { type: 'file', accept: '.jff,.json,.xml', multiple: true, style: 'display:none', onchange: () => {
      this.openFiles([...input.files]);
      input.remove();
    } });
    document.body.append(input);
    input.click();
  }

  async openFiles(files) {
    for (const file of files) {
      try {
        const text = await file.text();
        const title = file.name.replace(/\.[^.]+$/, '');
        await this.openText(text, title);
      } catch (error) {
        toast(`${file.name}: ${error.message}`, 'error');
      }
    }
  }

  /** Opens `.jff` XML or JSON text (full or compact document) in a new tab. */
  async openText(text, title) {
    let doc;
    if (text.trimStart().startsWith('<')) doc = (await this.api({ op: 'open_jff', xml: text })).document;
    else doc = await normalizeDocument(this.api, JSON.parse(text));
    return this.openTab(doc, { title });
  }

  async openFromHash() {
    let raw;
    try {
      raw = documentFromHash(location.hash);
    } catch (error) {
      toast(t('file.bad_link', { message: error.message }), 'error');
      return false;
    }
    if (!raw) return false;
    try {
      this.openTab(await normalizeDocument(this.api, raw), { title: t('file.shared') });
      return true;
    } catch (error) {
      toast(error.message, 'error');
      return false;
    }
  }

  // ------------------------------------------------------------ the host (chat)

  /** A short, readable message with the compact JSON of the automaton, so the AI sees the edits. */
  chatMessage(tab) {
    const compact = JSON.stringify(compactOf(tab.doc));
    return `${t('chat.message_intro', { title: tab.title })}\n${describe(tab.doc)}\n\`\`\`json\n${compact}\n\`\`\``;
  }

  async sendToChat(tab) {
    if (!tab.isAutomaton) return;
    try {
      await this.host.sendMessage(this.chatMessage(tab));
      toast(t('chat.sent'));
    } catch (error) {
      toast(t('chat.failed', { message: error.message }), 'error');
    }
  }

  async pushModelContext(tab) {
    if (!this.host.connected || !tab.isAutomaton || this.active !== tab) return;
    try {
      const structured = { document: compactOf(tab.doc) };
      if (this.host.mode === 'chatgpt') structured.language = this.languageChoice; // setWidgetState replaces the whole state
      await this.host.updateModelContext({ text: `${t('chat.context', { title: tab.title })} ${describe(tab.doc)}`, structured });
    } catch {
      /* the host may refuse context updates: the "Send to chat" button still works */
    }
  }

  /** `structuredContent` of a tool result (or ChatGPT's toolOutput): load the document, keep the extras. */
  async showToolResult(structured, { saved = null } = {}) {
    const { document: raw, extras } = splitStructured(structured);
    if (structured && typeof structured.viewerUrl === 'string') this.viewerUrl = structured.viewerUrl;
    let doc = null;
    const source = saved || raw;
    if (source) {
      try {
        doc = await normalizeDocument(this.api, source);
      } catch (error) {
        toast(error.message, 'error');
      }
    }
    const steps = Array.isArray(extras.steps) ? { title: t('steps.from_assistant'), steps: extras.steps, source: null } : null;
    delete extras.steps;
    const hasExtras = Object.keys(contentExtras(extras)).length > 0;
    const title = resultTitle(structured);
    const notation = resultNotation(structured);
    if (!doc && !hasExtras && !steps) return null;
    let tab = this.aiTab && this.tabList.includes(this.aiTab) && !this.aiTab.dirty ? this.aiTab : null;
    if (!tab && this.tabList.length === 1 && this.isPristine(this.tabList[0])) tab = this.tabList[0];
    if (tab) this.aiTab = tab;
    if (doc && tab) {
      const typeChanged = tab.type !== doc.type;
      if (typeChanged) this.destroyWorkspace(tab);
      tab.replace(doc, 'host');
      if (typeChanged) this.buildWorkspace(tab);
      tab.title = title || t('host.from_assistant');
      tab.setNotationOverride(doc.type === 'turing' ? notation : null);
      tab.setCaption(title);
      this.renderTabs();
      tab.setExtras(hasExtras ? extras : null);
      tab.setSteps(steps);
      this.activate(tab);
    } else if (doc) {
      tab = this.openTab(doc, { title: title || t('host.from_assistant'), steps, extras: hasExtras ? extras : null });
      if (doc.type === 'turing') tab.setNotationOverride(notation);
      tab.setCaption(title);
      this.aiTab = tab;
    } else if (tab || this.active) {
      tab = tab || this.active;
      tab.setExtras(hasExtras ? extras : null);
      if (steps) tab.setSteps(steps);
    }
    if (tab && hasExtras) this.showPanel(tab, 'assistant', undefined, { focus: false });
    else if (tab && steps) this.showPanel(tab, 'steps', undefined, { focus: false });
    return tab;
  }

  /** An untouched empty automaton (the tab opened at startup). */
  isPristine(tab) {
    if (tab.dirty || !tab.isAutomaton || tab.history.canUndo) return false;
    const machine = tab.doc.automaton || tab.doc;
    return Object.keys(machine.states).length === 0;
  }

  handleContext(context) {
    this.hostContext = { ...this.hostContext, ...context };
    this.applyTheme();
    // the host locale is ignored; the only host input is the language this widget saved in ChatGPT's widget state
    const saved = context.widgetState && context.widgetState.language;
    if (LANGUAGES.includes(saved) && saved !== this.languageChoice) {
      this.languageChoice = saved;
      const before = i18n.locale;
      this.applyLanguageChoice();
      if (this.mounted && before !== i18n.locale) this.rebuild();
    }
    if (context.containerDimensions && context.containerDimensions.maxHeight) {
      document.documentElement.style.setProperty('--embedded-height', `${Math.min(720, context.containerDimensions.maxHeight)}px`);
    }
  }

  /** The error banner for a page that cannot run the engine at all. */
  showFatal(message) {
    clear(this.main).append(h('div', { class: 'fatal', role: 'alert' }, h('h2', {}, t('fatal.title')), h('p', {}, message)));
  }
}
