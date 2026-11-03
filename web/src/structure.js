// "Structure" panel: states and transitions as tables. It is the keyboard / screen-reader way to do
// everything the canvas does with the mouse (add, rename, mark initial/final, edit labels, delete).

import { h, clear } from './dom.js';
import { t } from './i18n.js';
import { registerPanel } from './registry.js';
import { askLabel } from './labels.js';
import {
  container, stateList, labelToText, realMachine,
  addState, removeState, setInitial, toggleFinal, renameState, setStateOutput, setStateLabel,
  addTransition, removeTransition, setTransitionLabel, ensureBlock, removeBlock, hasBlock,
} from './doc.js';

registerPanel({
  id: 'structure',
  title: 'panel.structure',
  appliesTo: ['automata'],
  order: 30,
  mount(root, tab) {
    const body = h('div', { class: 'panel-body' });
    root.append(body);

    const stateName = (c, id) => (c.states[id] ? c.states[id].name : String(id));

    function draw() {
      clear(body);
      const c = container(tab.doc, tab.path);
      const states = stateList(tab.doc, tab.path);
      const moore = tab.type === 'moore';
      const turing = tab.type === 'turing';
      const notation = turing ? tab.app.notation : undefined;

      const stateRows = states.map((s) =>
        h(
          'tr',
          {},
          h('td', {}, h('input', { type: 'text', value: s.name, class: 'narrow', 'aria-label': t('structure.name_of', { name: s.name }), onchange: (e) => tab.edit((d, p) => renameState(d, s.id, e.target.value, p)).then((ok) => ok || draw()) })),
          h('td', {}, h('input', { type: 'radio', name: `initial-${tab.id}`, checked: c.initial === s.id, 'aria-label': t('structure.initial_of', { name: s.name }), onchange: () => tab.edit((d, p) => setInitial(d, s.id, p)) })),
          h('td', {}, h('input', { type: 'checkbox', checked: c.finals.includes(s.id), 'aria-label': t('structure.final_of', { name: s.name }), onchange: () => tab.edit((d, p) => toggleFinal(d, s.id, p)) })),
          moore ? h('td', {}, h('input', { type: 'text', value: s.output || '', class: 'narrow', 'aria-label': t('structure.output_of', { name: s.name }), onchange: (e) => tab.edit((d, p) => setStateOutput(d, s.id, e.target.value === 'λ' ? '' : e.target.value, p)) })) : null,
          h('td', {}, h('input', { type: 'text', value: s.label || '', class: 'narrow', 'aria-label': t('structure.label_of', { name: s.name }), onchange: (e) => tab.edit((d, p) => setStateLabel(d, s.id, e.target.value, p)) })),
          turing
            ? h(
                'td',
                {},
                hasBlock(tab.doc, s.id, tab.path)
                  ? [
                      h('button', { type: 'button', onclick: () => tab.enterBlock(s.id) }, t('block.enter')),
                      h('button', { type: 'button', onclick: () => tab.edit((d, p) => removeBlock(d, s.id, p)) }, t('block.remove')),
                    ]
                  : h('button', { type: 'button', onclick: () => tab.edit((d, p) => ensureBlock(d, s.id, p)) }, t('block.make')),
              )
            : null,
          h('td', {}, h('button', { type: 'button', 'aria-label': t('structure.delete_state', { name: s.name }), onclick: () => tab.edit((d, p) => removeState(d, s.id, p)) }, '✕')),
        ),
      );

      const transitionRows = c.transitions.map((x, i) =>
        h(
          'tr',
          {},
          h('td', {}, stateName(c, x.from)),
          h('td', { class: 'mono' }, labelToText(tab.type, x.label, notation)),
          h('td', {}, stateName(c, x.to)),
          h(
            'td',
            {},
            h('button', { type: 'button', onclick: async () => {
              const label = await askLabel({ api: tab.api, type: tab.type, tapes: realMachine(tab.doc, tab.path).tapes || 1, notation, text: labelToText(tab.type, x.label, notation), title: t('label.edit_transition', { from: stateName(c, x.from), to: stateName(c, x.to) }) });
              if (label) tab.edit((d, p) => setTransitionLabel(d, i, label, p));
            } }, t('structure.edit')),
            h('button', { type: 'button', 'aria-label': t('structure.delete_transition', { n: i + 1 }), onclick: () => tab.edit((d, p) => removeTransition(d, i, p)) }, '✕'),
          ),
        ),
      );

      const optionList = (value) => states.map((s) => h('option', { value: s.id, selected: s.id === value }, s.name));
      const from = h('select', { 'aria-label': t('structure.from') }, optionList());
      const to = h('select', { 'aria-label': t('structure.to') }, optionList());
      const addTransitionForm = h(
        'div',
        { class: 'button-row' },
        from,
        '→',
        to,
        h('button', { type: 'button', disabled: !states.length, onclick: async () => {
          const label = await askLabel({ api: tab.api, type: tab.type, tapes: realMachine(tab.doc, tab.path).tapes || 1, notation: tab.app.notation, title: t('label.new_transition', { from: c.states[from.value].name, to: c.states[to.value].name }) });
          if (label) tab.edit((d, p) => addTransition(d, Number(from.value), Number(to.value), label, p));
        } }, t('structure.add_transition')),
      );

      body.append(
        h('h3', {}, t('structure.states')),
        h(
          'table',
          { class: 'configs' },
          h('thead', {}, h('tr', {}, h('th', { scope: 'col' }, t('state.name')), h('th', { scope: 'col' }, t('word.initial')), h('th', { scope: 'col' }, t('word.final')), moore ? h('th', { scope: 'col' }, t('sim.col.output')) : null, h('th', { scope: 'col' }, t('state.label')), turing ? h('th', { scope: 'col' }, t('block.title')) : null, h('th', {}))),
          h('tbody', {}, stateRows),
        ),
        h('div', { class: 'button-row' }, h('button', { type: 'button', onclick: () => tab.edit((d, p) => {
          const n = Object.keys(container(d, p).states).length;
          addState(d, 60 + (n % 6) * 90, 60 + Math.floor(n / 6) * 90, p);
        }) }, t('structure.add_state'))),
        h('h3', {}, t('structure.transitions')),
        c.transitions.length
          ? h('table', { class: 'configs' }, h('thead', {}, h('tr', {}, h('th', { scope: 'col' }, t('structure.from')), h('th', { scope: 'col' }, t('structure.label')), h('th', { scope: 'col' }, t('structure.to')), h('th', {}))), h('tbody', {}, transitionRows))
          : h('p', { class: 'muted' }, t('structure.no_transitions')),
        addTransitionForm,
      );
    }

    draw();
    const offs = [tab.on('doc', (e) => e.cause !== 'flags' && draw()), tab.on('notation', draw)];
    return { destroy: () => offs.forEach((off) => off()) };
  },
});
