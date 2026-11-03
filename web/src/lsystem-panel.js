// L-system environment (JFLAP's "L-System" window): axiom, rule table, drawing parameters, a depth slider and
// the turtle drawing rendered by the engine (`lsystem_render`), with SVG export.

import { h, clear, debounce, download } from './dom.js';
import { t, registerStrings } from './i18n.js';
import { registerPanel, registerAction } from './registry.js';
import { toast, textDialog } from './dialogs.js';
import { svgImage } from './widgets.js';
import { lsystemForm, lsystemCompact, LSYSTEM_EXAMPLE } from './lsystem-model.js';

registerStrings({
  en: {
    'panel.lsystem': 'L-system',
    'file.new_lsystem': 'New L-system',
    'lsystem.axiom': 'Axiom',
    'lsystem.axiom_hint': 'Symbols separated by spaces, for example: g',
    'lsystem.rules': 'Rules',
    'lsystem.col.lhs': 'Symbol',
    'lsystem.col.rhs': 'Replacement',
    'lsystem.lhs_of': 'Symbol of rule {n}',
    'lsystem.rhs_of': 'Replacement of rule {n}',
    'lsystem.add_rule': 'Add rule',
    'lsystem.remove_rule': 'Remove rule {n}',
    'lsystem.parameters': 'Parameters',
    'lsystem.col.name': 'Name',
    'lsystem.col.value': 'Value',
    'lsystem.name_of': 'Name of parameter {n}',
    'lsystem.value_of': 'Value of parameter {n}',
    'lsystem.add_parameter': 'Add parameter',
    'lsystem.remove_parameter': 'Remove parameter {n}',
    'lsystem.depth': 'Depth (rewrite levels)',
    'lsystem.depth_value': 'Level {n}',
    'lsystem.example': 'Load the example',
    'lsystem.export': 'Export SVG',
    'lsystem.drawing': 'Drawing',
    'lsystem.drawing_alt': 'Turtle drawing of the L-system at level {n}',
    'lsystem.size': 'The expansion has {n} symbols.',
    'lsystem.expansion': 'Expansion',
    'lsystem.help': 'Symbols: g draws forward, f moves without drawing, + and - turn, [ and ] save and restore the position, { and } fill a polygon. Parameters: angle (degrees), distance, lineWidth, color…',
    'lsystem.rendering': 'Drawing…',
    'lsystem.error': 'Cannot draw: {message}',
    'lsystem.nothing': 'Nothing to export yet.',
    'lsystem.exported': 'Saved {name}',
  },
  es: {
    'panel.lsystem': 'Sistema L',
    'file.new_lsystem': 'Nuevo sistema L',
    'lsystem.axiom': 'Axioma',
    'lsystem.axiom_hint': 'Símbolos separados por espacios, por ejemplo: g',
    'lsystem.rules': 'Reglas',
    'lsystem.col.lhs': 'Símbolo',
    'lsystem.col.rhs': 'Reemplazo',
    'lsystem.lhs_of': 'Símbolo de la regla {n}',
    'lsystem.rhs_of': 'Reemplazo de la regla {n}',
    'lsystem.add_rule': 'Agregar regla',
    'lsystem.remove_rule': 'Quitar la regla {n}',
    'lsystem.parameters': 'Parámetros',
    'lsystem.col.name': 'Nombre',
    'lsystem.col.value': 'Valor',
    'lsystem.name_of': 'Nombre del parámetro {n}',
    'lsystem.value_of': 'Valor del parámetro {n}',
    'lsystem.add_parameter': 'Agregar parámetro',
    'lsystem.remove_parameter': 'Quitar el parámetro {n}',
    'lsystem.depth': 'Profundidad (niveles de reescritura)',
    'lsystem.depth_value': 'Nivel {n}',
    'lsystem.example': 'Cargar el ejemplo',
    'lsystem.export': 'Exportar SVG',
    'lsystem.drawing': 'Dibujo',
    'lsystem.drawing_alt': 'Dibujo de la tortuga del sistema L en el nivel {n}',
    'lsystem.size': 'La expansión tiene {n} símbolos.',
    'lsystem.expansion': 'Expansión',
    'lsystem.help': 'Símbolos: g dibuja hacia adelante, f avanza sin dibujar, + y - giran, [ y ] guardan y restauran la posición, { y } rellenan un polígono. Parámetros: angle (grados), distance, lineWidth, color…',
    'lsystem.rendering': 'Dibujando…',
    'lsystem.error': 'No se puede dibujar: {message}',
    'lsystem.nothing': 'Todavía no hay nada para exportar.',
    'lsystem.exported': 'Se guardó {name}',
  },
});

const MAX_DEPTH = 12;

registerPanel({
  id: 'lsystem-editor',
  title: 'panel.lsystem',
  appliesTo: ['lsystem'],
  order: 10,
  mount(root, tab) {
    const body = h('div', { class: 'panel-body lsystem-panel' });
    root.append(body);
    let form = lsystemForm(tab.doc);
    let depth = 3;
    let svg = '';
    let token = 0;
    let rebuilding = false;
    const picture = h('div', { class: 'lsystem-picture', 'aria-live': 'polite' });
    const info = h('p', { class: 'muted', role: 'status' });
    const expansion = h('pre', { class: 'description mono expansion' });
    const problem = h('p', { class: 'form-error', role: 'alert' });

    /** The form rows -> the compact document -> normalize -> an undoable commit. */
    async function push() {
      problem.textContent = '';
      try {
        const { document } = await tab.api({ op: 'normalize', document: lsystemCompact(form) });
        rebuilding = true;
        await tab.commit(document);
        rebuilding = false;
        render();
      } catch (error) {
        rebuilding = false;
        problem.textContent = error.message;
      }
    }

    const render = debounce(async () => {
      const mine = ++token;
      let compact;
      try {
        compact = lsystemCompact(form);
      } catch (error) {
        clear(picture);
        info.textContent = '';
        return;
      }
      info.textContent = t('lsystem.rendering');
      try {
        const r = await tab.api({ op: 'lsystem_render', document: compact, depth });
        if (mine !== token) return;
        svg = r.svg;
        clear(picture).append(svgImage(r.svg, t('lsystem.drawing_alt', { n: r.depth }), 'engine-svg lsystem-svg'));
        info.textContent = t('lsystem.size', { n: r.expansion_length });
        expansion.textContent = r.expansion === null || r.expansion === undefined ? '' : r.expansion;
        expansion.hidden = !r.expansion;
        problem.textContent = '';
      } catch (error) {
        if (mine !== token) return;
        svg = '';
        clear(picture);
        info.textContent = '';
        problem.textContent = t('lsystem.error', { message: error.message });
      }
    }, 150);

    function exportSvg() {
      if (!svg) {
        toast(t('lsystem.nothing'), 'error');
        return;
      }
      const name = `${(tab.title || 'lsystem').replace(/[^\p{L}\p{N}._ -]+/gu, '_').trim() || 'lsystem'}.svg`;
      if (download(name, svg, 'image/svg+xml')) toast(t('lsystem.exported', { name }));
      else textDialog({ title: name, text: svg, hint: t('file.download_blocked') });
    }

    const cell = (value, label, onchange, wide = false) => h('input', { type: 'text', value, class: `mono ${wide ? 'wide' : 'narrow'}`, 'aria-label': label, spellcheck: 'false', autocomplete: 'off', onchange: (e) => onchange(e.target.value) });

    function draw() {
      clear(body);
      const axiom = h('input', { type: 'text', value: form.axiom, id: `axiom-${tab.id}`, class: 'mono wide', spellcheck: 'false', onchange: (e) => ((form.axiom = e.target.value), push()) });
      const slider = h('input', { type: 'range', min: 0, max: MAX_DEPTH, value: depth, id: `depth-${tab.id}`, 'aria-valuetext': t('lsystem.depth_value', { n: depth }), oninput: (e) => {
        depth = Number(e.target.value);
        label.textContent = t('lsystem.depth_value', { n: depth });
        e.target.setAttribute('aria-valuetext', label.textContent);
        render();
      } });
      const label = h('output', { for: slider.id }, t('lsystem.depth_value', { n: depth }));
      body.append(
        h('p', { class: 'muted' }, t('lsystem.help')),
        h('div', { class: 'form-row' }, h('label', { for: axiom.id }, t('lsystem.axiom')), axiom, h('small', {}, t('lsystem.axiom_hint'))),
        h('h3', {}, t('lsystem.rules')),
        h(
          'table',
          { class: 'configs' },
          h('caption', { class: 'sr-only' }, t('lsystem.rules')),
          h('thead', {}, h('tr', {}, h('th', {}, t('lsystem.col.lhs')), h('th', {}), h('th', {}, t('lsystem.col.rhs')), h('th', {}))),
          h(
            'tbody',
            {},
            form.rules.map((rule, i) =>
              h(
                'tr',
                {},
                h('td', {}, cell(rule.lhs, t('lsystem.lhs_of', { n: i + 1 }), (v) => ((form.rules[i].lhs = v), push()))),
                h('td', { 'aria-hidden': 'true' }, '→'),
                h('td', {}, cell(rule.rhs, t('lsystem.rhs_of', { n: i + 1 }), (v) => ((form.rules[i].rhs = v), push()), true)),
                h('td', {}, h('button', { type: 'button', 'aria-label': t('lsystem.remove_rule', { n: i + 1 }), onclick: () => (form.rules.splice(i, 1), draw(), push()) }, '✕')),
              ),
            ),
          ),
        ),
        h('div', { class: 'button-row' }, h('button', { type: 'button', onclick: () => (form.rules.push({ lhs: '', rhs: '' }), draw()) }, t('lsystem.add_rule'))),
        h('h3', {}, t('lsystem.parameters')),
        h(
          'table',
          { class: 'configs' },
          h('caption', { class: 'sr-only' }, t('lsystem.parameters')),
          h('thead', {}, h('tr', {}, h('th', {}, t('lsystem.col.name')), h('th', {}, t('lsystem.col.value')), h('th', {}))),
          h(
            'tbody',
            {},
            form.parameters.map((p, i) =>
              h(
                'tr',
                {},
                h('td', {}, cell(p.name, t('lsystem.name_of', { n: i + 1 }), (v) => ((form.parameters[i].name = v), push()))),
                h('td', {}, cell(p.value, t('lsystem.value_of', { n: i + 1 }), (v) => ((form.parameters[i].value = v), push()))),
                h('td', {}, h('button', { type: 'button', 'aria-label': t('lsystem.remove_parameter', { n: i + 1 }), onclick: () => (form.parameters.splice(i, 1), draw(), push()) }, '✕')),
              ),
            ),
          ),
        ),
        h('div', { class: 'button-row' }, h('button', { type: 'button', onclick: () => (form.parameters.push({ name: '', value: '' }), draw()) }, t('lsystem.add_parameter'))),
        problem,
        h('div', { class: 'form-row' }, h('label', { for: slider.id }, t('lsystem.depth')), h('div', { class: 'form-input' }, slider, label)),
        h('div', { class: 'button-row' }, h('button', { type: 'button', class: 'primary', onclick: exportSvg }, t('lsystem.export')), h('button', { type: 'button', onclick: () => ((form = JSON.parse(JSON.stringify(LSYSTEM_EXAMPLE))), draw(), push()) }, t('lsystem.example'))),
        h('h3', {}, t('lsystem.drawing')),
        info,
        picture,
        h('h3', {}, t('lsystem.expansion')),
        expansion,
      );
      render();
    }

    draw();
    const off = tab.on('doc', (e) => {
      if (rebuilding) return;
      if (e.cause === 'edit' || e.cause === 'undo' || e.cause === 'redo' || e.cause === 'replace' || e.cause === 'host') {
        form = lsystemForm(tab.doc);
        draw();
      }
    });
    return { destroy: off, focus: () => body.querySelector('input')?.focus(), exportSvg };
  },
});

registerAction({
  id: 'file.new.lsystem',
  menu: 'file',
  label: 'file.new_lsystem',
  order: 5.2,
  async run(tab) {
    try {
      const { document } = await tab.api({ op: 'normalize', document: { type: 'lsystem', axiom: LSYSTEM_EXAMPLE.axiom, rules: LSYSTEM_EXAMPLE.rules.map((r) => `${r.lhs} -> ${r.rhs}`), parameters: Object.fromEntries(LSYSTEM_EXAMPLE.parameters.map((p) => [p.name, p.value])) } });
      tab.app.newTab(document);
    } catch (error) {
      tab.notify(error.message, 'error');
    }
  },
});
registerAction({
  id: 'file.export_svg.lsystem',
  menu: 'file',
  label: 'lsystem.export',
  order: 13,
  appliesTo: ['lsystem'],
  run: (tab) => tab.app.showPanel(tab, 'lsystem-editor', 'exportSvg'),
});
