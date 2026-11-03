// Pure side of the L-system panel: the full document <-> the form rows (axiom, rules, parameters).

import { t, registerStrings } from './i18n.js';

registerStrings({
  en: {
    'lsystem.err.axiom': 'Write the axiom: the symbols the drawing starts from, such as "g".',
    'lsystem.err.rule_left': 'A rule has a right side but no left side.',
    'lsystem.err.rule_symbol': 'The left side of a rule is one symbol; "{lhs}" has several.',
    'lsystem.err.parameter': 'A parameter needs a name (for example angle).',
  },
  es: {
    'lsystem.err.axiom': 'Escribí el axioma: los símbolos desde los que empieza el dibujo, por ejemplo "g".',
    'lsystem.err.rule_left': 'Una regla tiene lado derecho pero no lado izquierdo.',
    'lsystem.err.rule_symbol': 'El lado izquierdo de una regla es un símbolo; "{lhs}" tiene varios.',
    'lsystem.err.parameter': 'Un parámetro necesita un nombre (por ejemplo angle).',
  },
});

/** A Koch-like curve that draws with `g` (JFLAP draws with `g`, moves with `f`, turns with `+` and `-`). */
export const LSYSTEM_EXAMPLE = {
  axiom: 'g',
  rules: [{ lhs: 'g', rhs: 'g + g - g - g + g' }],
  parameters: [
    { name: 'angle', value: '90' },
    { name: 'distance', value: '4' },
  ],
};

/** The form rows of a full L-system document (`axiom` and replacements are arrays of symbols). */
export function lsystemForm(doc) {
  return {
    axiom: (doc.axiom || []).join(' '),
    rules: Object.entries(doc.rules || {}).flatMap(([lhs, replacements]) => replacements.map((symbols) => ({ lhs, rhs: symbols.join(' ') }))),
    parameters: (doc.parameters || []).map(([name, value]) => ({ name, value })),
  };
}

/** The compact document (`{type, axiom, rules: ['g -> g + g'], parameters}`) the engine reads; blank rows vanish. */
export function lsystemCompact(form) {
  const axiom = String(form.axiom || '').trim();
  if (!axiom) throw new Error(t('lsystem.err.axiom'));
  const rules = [];
  for (const row of form.rules) {
    const lhs = String(row.lhs || '').trim();
    const rhs = String(row.rhs || '').trim();
    if (!lhs && !rhs) continue;
    if (!lhs) throw new Error(t('lsystem.err.rule_left'));
    if (/\s/.test(lhs)) throw new Error(t('lsystem.err.rule_symbol', { lhs }));
    rules.push(`${lhs} -> ${rhs}`);
  }
  const parameters = {};
  for (const row of form.parameters) {
    const name = String(row.name || '').trim();
    const value = String(row.value || '').trim();
    if (!name && !value) continue;
    if (!name) throw new Error(t('lsystem.err.parameter'));
    parameters[name] = value;
  }
  return { type: 'lsystem', axiom, rules, parameters };
}
