// Menu actions in the order of JFLAP's menus: File, Edit, Input, Test, Convert, Layout, View, Help.
// An action is `{id, menu, label, appliesTo, order, shortcut, enabled(ctx), checked(ctx), run(ctx)}`;
// `ctx` is the active tab (see tab.js) and `ctx.app` the shell.

import { h } from './dom.js';
import { t } from './i18n.js';
import { registerAction } from './registry.js';
import { download } from './dom.js';
import { formDialog, messageDialog, textDialog, toast } from './dialogs.js';
import { emptyDocument, machineDocument } from './doc.js';
import { simSettings } from './sim.js';
import { shareLink } from './share.js';

const GRAPH_TYPES = ['automata'];
const FA = ['fa'];

const safeName = (title) => (title || 'automaton').replace(/[^\p{L}\p{N}._ -]+/gu, '_').trim() || 'automaton';

/** Runs `work`, showing the engine's error as a toast instead of failing silently. */
export async function guarded(tab, work) {
  try {
    return await work();
  } catch (error) {
    tab.notify(error.message, 'error');
    return undefined;
  }
}

export async function copyText(text, title) {
  try {
    await navigator.clipboard.writeText(text);
    toast(t('file.copied'));
  } catch {
    // clipboard-write is not granted inside most host iframes: let the user copy by hand
    await textDialog({ title, text, hint: t('file.copy_by_hand') });
  }
}

function saveFile(filename, content, mime) {
  if (download(filename, content, mime)) toast(t('file.saved', { name: filename }));
  else textDialog({ title: filename, text: content, hint: t('file.download_blocked') });
}

// ------------------------------------------------------------------------------------ File

const newTypes = [
  ['fa', 'file.new_fa'],
  ['pda', 'file.new_pda'],
  ['turing', 'file.new_tm'],
  ['mealy', 'file.new_mealy'],
  ['moore', 'file.new_moore'],
];
newTypes.forEach(([type, label], i) => {
  registerAction({
    id: `file.new.${type}`,
    menu: 'file',
    label,
    order: i,
    async run(tab) {
      let tapes = 1;
      if (type === 'turing') {
        const answer = await formDialog({
          title: t('file.new_tm'),
          fields: [{ name: 'tapes', label: t('file.tapes'), type: 'number', value: '1', min: 1, max: 5 }],
          validate: (v) => {
            const n = Number(v.tapes);
            if (!Number.isInteger(n) || n < 1 || n > 5) throw new Error(t('file.tapes_range'));
          },
        });
        if (!answer) return;
        tapes = Number(answer.tapes);
      }
      tab.app.newTab(emptyDocument(type, tapes));
    },
  });
});

registerAction({ id: 'file.open', menu: 'file', label: 'file.open', order: 10, separator: true, shortcut: 'Ctrl+O', run: (tab) => tab.app.pickFiles() });
registerAction({
  id: 'file.save_jff',
  menu: 'file',
  label: 'file.save_jff',
  order: 11,
  shortcut: 'Ctrl+S',
  run: (tab) =>
    guarded(tab, async () => {
      const r = await tab.api({ op: 'save_jff', document: tab.doc });
      saveFile(`${safeName(tab.title)}.jff`, r.xml, 'application/xml');
    }),
});
registerAction({
  id: 'file.save_json',
  menu: 'file',
  label: 'file.save_json',
  order: 12,
  run: (tab) => saveFile(`${safeName(tab.title)}.json`, JSON.stringify(tab.doc, null, 2), 'application/json'),
});
registerAction({
  id: 'file.export_svg',
  menu: 'file',
  label: 'file.export_svg',
  order: 13,
  appliesTo: GRAPH_TYPES,
  run: (tab) =>
    guarded(tab, async () => {
      const r = await tab.api({ op: 'render', document: machineDocument(tab.doc, tab.path), format: 'svg' });
      saveFile(`${safeName(tab.title)}.svg`, r.content, 'image/svg+xml');
    }),
});
registerAction({
  id: 'file.share',
  menu: 'file',
  label: 'file.share',
  order: 14,
  run: (tab) => copyText(shareLink(tab.app.pageUrl(), tab.doc), t('file.share')),
});
registerAction({ id: 'file.close', menu: 'file', label: 'file.close', order: 20, separator: true, shortcut: 'Ctrl+W', run: (tab) => tab.app.closeTab(tab) });

// ------------------------------------------------------------------------------------ Edit

registerAction({ id: 'edit.undo', menu: 'edit', label: 'edit.undo', order: 0, shortcut: 'Ctrl+Z', enabled: (tab) => tab.history.canUndo, run: (tab) => tab.undo() });
registerAction({ id: 'edit.redo', menu: 'edit', label: 'edit.redo', order: 1, shortcut: 'Ctrl+Y', enabled: (tab) => tab.history.canRedo, run: (tab) => tab.redo() });
registerAction({
  id: 'edit.select_all',
  menu: 'edit',
  label: 'edit.select_all',
  order: 2,
  separator: true,
  appliesTo: GRAPH_TYPES,
  shortcut: 'Ctrl+A',
  run: (tab) => tab.canvas && tab.canvas.selectAll(),
});
registerAction({
  id: 'edit.delete',
  menu: 'edit',
  label: 'edit.delete',
  order: 3,
  appliesTo: GRAPH_TYPES,
  enabled: (tab) => tab.selection.states.length > 0 || tab.selection.transition != null,
  run: (tab) => tab.canvas && tab.canvas.removeSelection(),
});

// ------------------------------------------------------------------------------------ Input

const sim = (tab, method) => tab.app.showPanel(tab, 'simulate', method);

registerAction({ id: 'input.step_closure', menu: 'input', label: 'input.step_closure', order: 0, appliesTo: ['fa', 'pda'], run: (tab) => { simSettings(tab).mode = 'closure'; return sim(tab, 'step'); } });
registerAction({ id: 'input.step_state', menu: 'input', label: 'input.step_state', order: 1, appliesTo: ['fa', 'pda'], run: (tab) => { simSettings(tab).mode = 'step_by_state'; return sim(tab, 'step'); } });
registerAction({ id: 'input.step', menu: 'input', label: 'input.step', order: 2, appliesTo: ['turing', 'mealy', 'moore'], run: (tab) => sim(tab, 'step') });
registerAction({ id: 'input.run', menu: 'input', label: 'input.run', order: 3, appliesTo: GRAPH_TYPES, run: (tab) => sim(tab, 'run') });
registerAction({
  id: 'input.multiple',
  menu: 'input',
  label: 'input.multiple',
  order: 4,
  appliesTo: GRAPH_TYPES,
  run: (tab) => tab.app.showPanel(tab, 'multiple'),
});

// ------------------------------------------------------------------------------------ Test

registerAction({
  id: 'test.nondeterminism',
  menu: 'test',
  label: 'test.highlight_nondeterminism',
  order: 0,
  appliesTo: GRAPH_TYPES,
  checked: (tab) => tab.flags.nondeterminism,
  run: (tab) => tab.setFlag('nondeterminism', !tab.flags.nondeterminism),
});
registerAction({
  id: 'test.lambda',
  menu: 'test',
  label: 'test.highlight_lambda',
  order: 1,
  appliesTo: GRAPH_TYPES,
  checked: (tab) => tab.flags.lambda,
  run: (tab) => tab.setFlag('lambda', !tab.flags.lambda),
});
registerAction({ id: 'test.analysis', menu: 'test', label: 'test.analysis', order: 2, appliesTo: GRAPH_TYPES, run: (tab) => tab.app.showPanel(tab, 'analysis') });

// ------------------------------------------------------------------------------------ Convert

/** Runs a `convert` and opens the result in a new tab, with the steps for the step viewer. */
export async function convertToTab(tab, to, titleKey) {
  return guarded(tab, async () => {
    const r = await tab.api({ op: 'convert', document: machineDocument(tab.doc, tab.path), to });
    const title = `${tab.title} → ${t(titleKey)}`;
    // `to`, `tree` and `sourceDoc` let the step viewer draw the richer views (subset table, tree, graphs)
    const steps = Array.isArray(r.steps) && r.steps.length ? { title: t(titleKey), steps: r.steps, source: tab, to, tree: r.tree || null, sourceDoc: tab.isAutomaton ? machineDocument(tab.doc, tab.path) : null } : null;
    const result = tab.openTab(r.document, { title, steps });
    if (steps) tab.app.showPanel(result, 'steps');
    return result;
  });
}

const conversions = [
  ['dfa', 'convert.dfa', FA, 0],
  ['minimal_dfa', 'convert.minimal', FA, 1],
  ['regex', 'convert.regex', FA, 2],
  ['regular_grammar', 'convert.grammar', FA, 3],
  ['complete', 'convert.trap', FA, 4],
  ['remove_useless_states', 'convert.remove_useless_states', ['fa', 'pda', 'mealy', 'moore'], 5],
  ['split_labels', 'convert.split_labels', FA, 6],
  ['cfg', 'convert.cfg', ['pda'], 7],
  ['unrestricted_grammar', 'convert.unrestricted', ['turing'], 8],
  ['nfa', 'convert.nfa', ['regex'], 9],
  ['dfa', 'convert.dfa', ['regex'], 10],
  ['minimal_dfa', 'convert.minimal', ['regex', 'grammar'], 11],
  ['fa', 'convert.fa', ['grammar'], 12],
  ['pda_ll', 'convert.pda_ll', ['grammar'], 13],
  ['pda_lr', 'convert.pda_lr', ['grammar'], 14],
];
for (const [to, label, types, order] of conversions) {
  registerAction({ id: `convert.${to}.${types.join('-')}`, menu: 'convert', label, order, appliesTo: types, run: (tab) => convertToTab(tab, to, label) });
}

/** Asks which other open tab of the same type to use. Resolves with that tab or null. */
async function pickOtherTab(tab, title) {
  const others = tab.app.tabs().filter((x) => x !== tab && x.type === tab.type);
  if (!others.length) {
    tab.notify(t('convert.need_other'), 'error');
    return null;
  }
  const answer = await formDialog({
    title,
    fields: [{ name: 'tab', label: t('convert.other_tab'), type: 'select', options: others.map((x) => [x.id, x.title]), value: others[0].id }],
  });
  return answer ? others.find((x) => String(x.id) === String(answer.tab)) : null;
}

registerAction({
  id: 'convert.combine',
  menu: 'convert',
  label: 'convert.combine',
  order: 20,
  separator: true,
  appliesTo: FA,
  run: async (tab) => {
    const other = await pickOtherTab(tab, t('convert.combine'));
    if (!other) return;
    guarded(tab, async () => {
      const r = await tab.api({ op: 'combine', a: tab.doc, b: other.doc });
      tab.openTab(r.document, { title: t('convert.combined', { a: tab.title, b: other.title }) });
    });
  },
});
registerAction({
  id: 'convert.equivalent',
  menu: 'convert',
  label: 'convert.equivalent',
  order: 21,
  appliesTo: FA,
  run: async (tab) => {
    const other = await pickOtherTab(tab, t('convert.equivalent'));
    if (!other) return;
    guarded(tab, async () => {
      const r = await tab.api({ op: 'equivalent', a: tab.doc, b: other.doc });
      const text = r.equivalent
        ? t('convert.are_equivalent', { a: tab.title, b: other.title })
        : t('convert.not_equivalent', { a: tab.title, b: other.title, word: r.counterexample === '' ? 'λ' : r.counterexample });
      await messageDialog({ title: t('convert.equivalent'), body: h('p', { role: 'status' }, text) });
    });
  },
});

// ------------------------------------------------------------------------------------ Layout

const layouts = ['circle', 'gem', 'random', 'spiral', 'tree', 'tree-degree', 'two-circle'];
layouts.forEach((algorithm, order) => {
  registerAction({
    id: `layout.${algorithm}`,
    menu: 'layout',
    label: `layout.${algorithm}`,
    order,
    appliesTo: GRAPH_TYPES,
    enabled: (tab) => tab.path.length === 0,
    run: (tab) =>
      guarded(tab, async () => {
        const r = await tab.api({ op: 'layout', document: tab.doc, algorithm });
        await tab.commit(r.document);
      }),
  });
});

// ------------------------------------------------------------------------------------ View

registerAction({ id: 'view.zoom_in', menu: 'view', label: 'view.zoom_in', order: 0, appliesTo: GRAPH_TYPES, shortcut: 'Ctrl+=', run: (tab) => tab.canvas && tab.canvas.setZoom(tab.canvas.zoom * 1.25) });
registerAction({ id: 'view.zoom_out', menu: 'view', label: 'view.zoom_out', order: 1, appliesTo: GRAPH_TYPES, shortcut: 'Ctrl+-', run: (tab) => tab.canvas && tab.canvas.setZoom(tab.canvas.zoom / 1.25) });
registerAction({ id: 'view.zoom_reset', menu: 'view', label: 'view.zoom_reset', order: 2, appliesTo: GRAPH_TYPES, shortcut: 'Ctrl+0', run: (tab) => tab.canvas && tab.canvas.setZoom(1) });
registerAction({ id: 'view.zoom_fit', menu: 'view', label: 'view.zoom_fit', order: 2.5, appliesTo: GRAPH_TYPES, shortcut: 'Ctrl+9', run: (tab) => tab.canvas && tab.canvas.fit() });
registerAction({ id: 'view.exit_block', menu: 'view', label: 'block.exit', order: 3, separator: true, appliesTo: ['turing'], enabled: (tab) => tab.path.length > 0, run: (tab) => tab.exitBlock() });
for (const [index, code] of ['es', 'en'].entries()) {
  registerAction({
    id: `view.language.${code}`,
    menu: 'view',
    label: `view.language_${code}`,
    order: 20 + index,
    separator: index === 0,
    checked: (tab) => tab.app.languageChoice === code,
    run: (tab) => tab.app.setLanguage(code),
  });
}

// ------------------------------------------------------------------------------------ Chat (only with a host)

registerAction({
  id: 'chat.send',
  menu: 'view',
  label: 'chat.send',
  order: 40,
  separator: true,
  appliesTo: GRAPH_TYPES,
  enabled: (tab) => Boolean(tab.app.host.connected),
  run: (tab) => tab.app.sendToChat(tab),
});

// ------------------------------------------------------------------------------------ Help

registerAction({
  id: 'help.shortcuts',
  menu: 'help',
  label: 'help.shortcuts',
  order: 0,
  run: () =>
    messageDialog({
      title: t('help.shortcuts'),
      body: h(
        'table',
        { class: 'configs' },
        h('tbody', {}, ['help.sc.tools', 'help.sc.delete', 'help.sc.rename', 'help.sc.arrows', 'help.sc.undo', 'help.sc.open', 'help.sc.zoom', 'help.sc.step', 'help.sc.run', 'help.sc.menu'].map((k) => h('tr', {}, h('td', { class: 'mono' }, t(`${k}.keys`)), h('td', {}, t(`${k}.what`))))),
      ),
    }),
});
registerAction({
  id: 'help.about',
  menu: 'help',
  label: 'help.about',
  order: 1,
  run: () => messageDialog({ title: 'Rflap', body: h('div', {}, h('p', {}, t('help.about_text')), h('p', {}, t('help.credit'))) }),
});
