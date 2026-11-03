// Registries of the app shell: panels (the side area of a tab) and actions (menu items and shortcuts).
// T12b adds the panels for grammars, regexes, conversions replay, parsing, design, L-systems and pumping
// lemma by calling `registerPanel` / `registerAction`; see web/README.md.

const panels = new Map();
const actions = new Map();

/**
 * Registers a panel.
 *
 *   registerPanel({
 *     id: 'grammar-editor',            // unique
 *     title: 'grammar.title',          // an i18n key (registerStrings) or plain text
 *     appliesTo: ['grammar'],          // document types; ['*'] = every type; ['automata'] = fa, pda, turing, mealy, moore
 *     order: 50,                       // optional, lower comes first (built-in panels use 10..60)
 *     visible(ctx) { return true; },   // optional: hide the panel for tabs where it makes no sense
 *     mount(container, ctx) { ... return { destroy() {} }; },   // called once per tab, the first time it is shown
 *   });
 *
 * `ctx` is the tab context (see app.js `TabContext`): doc, setDoc/commit, api, t, host, openTab, notify, on(...).
 */
export function registerPanel(def) {
  if (!def || !def.id || typeof def.mount !== 'function') throw new Error('a panel needs an id and mount(container, ctx)');
  panels.set(def.id, { order: 100, appliesTo: ['*'], ...def });
}

export const AUTOMATA_TYPES = ['fa', 'pda', 'turing', 'mealy', 'moore'];

export function appliesTo(def, type) {
  return def.appliesTo.some((a) => a === '*' || a === type || (a === 'automata' && AUTOMATA_TYPES.includes(type)));
}

export function panelsFor(type, tab) {
  return [...panels.values()].filter((p) => appliesTo(p, type) && (!p.visible || p.visible(tab))).sort((a, b) => a.order - b.order || a.id.localeCompare(b.id));
}

export function allPanels() {
  return [...panels.values()];
}

/**
 * Registers a menu action.
 *
 *   registerAction({
 *     id: 'convert.to-dfa', menu: 'convert',        // menus: file, edit, input, test, convert, layout, view, help (or a new id)
 *     label: 'convert.dfa',                         // i18n key or text
 *     appliesTo: ['fa'], order: 10, shortcut: 'Ctrl+Shift+D',
 *     enabled(ctx) { return true; }, checked(ctx) { return false; },   // optional
 *     run(ctx) { ... },
 *   });
 *
 * `ctx` is the active tab context; actions of file-level menus receive the same context
 * (`ctx.app` gives the shell: tabs, newTab, openTab...). `separator: true` adds a divider before the item.
 */
export function registerAction(def) {
  if (!def || !def.id || !def.menu) throw new Error('an action needs an id and a menu');
  actions.set(def.id, { order: 100, appliesTo: ['*'], ...def });
}

export function actionsFor(menu, type) {
  return [...actions.values()]
    .filter((a) => a.menu === menu && (type === undefined || appliesTo(a, type)))
    .sort((a, b) => a.order - b.order || a.id.localeCompare(b.id));
}

export function actionById(id) {
  return actions.get(id);
}

export function allActions() {
  return [...actions.values()];
}
