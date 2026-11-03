// The label form of a transition, per automaton type (JFLAP asks the same fields):
//   fa / moore : one field (the string read) with a λ button
//   pda        : input, pop, push
//   turing     : read, write and move for each tape
//   mealy      : input, output
// The form builds the compact label text; the engine parses it (`labelFromText`), so a wrong label is
// reported with the engine's own message and the dialog stays open.

import { labelFromText } from './doc.js';
import { formDialog } from './dialogs.js';
import { t } from './i18n.js';

const unlambda = (s) => (s === 'λ' || s === 'ε' ? '' : s);

/** Text -> field values of the form. */
export function fieldsFromText(type, text, tapes = 1) {
  const values = {};
  if (type === 'pda') {
    const m = /^([^,]*),([^;]*);(.*)$/s.exec(text) || [];
    values.input = unlambda(m[1] ?? '');
    values.pop = unlambda(m[2] ?? '');
    values.push = unlambda(m[3] ?? '');
  } else if (type === 'mealy') {
    const at = text.indexOf(';');
    values.input = unlambda(at < 0 ? text : text.slice(0, at));
    values.output = unlambda(at < 0 ? '' : text.slice(at + 1));
  } else if (type === 'turing') {
    const parts = text.split('|');
    for (let i = 0; i < tapes; i++) {
      const m = /^([^;]*);(.*),([LRS])$/s.exec(parts[i] || '') || [];
      values[`read${i}`] = m[1] ?? '';
      values[`write${i}`] = m[2] ?? '';
      values[`move${i}`] = m[3] ?? 'R';
    }
  } else {
    values.text = unlambda(text);
  }
  return values;
}

/** Field values -> label text. */
export function textFromFields(type, values, tapes = 1) {
  const lam = (s) => (s && s.trim() ? s.trim() : 'λ');
  if (type === 'pda') return `${lam(values.input)},${lam(values.pop)};${lam(values.push)}`;
  if (type === 'mealy') return `${lam(values.input)};${lam(values.output)}`;
  if (type === 'turing') {
    const blank = (s) => (s && s.trim() ? s.trim() : '_');
    return Array.from({ length: tapes }, (_, i) => `${blank(values[`read${i}`])};${blank(values[`write${i}`])},${values[`move${i}`] || 'S'}`).join('|');
  }
  return (values.text || '').trim();
}

function formFields(type, tapes, initial) {
  const f = (name, label, extra = {}) => ({ name, label, value: initial[name] ?? '', ...extra });
  if (type === 'pda') {
    return [f('input', t('label.input'), { lambda: true }), f('pop', t('label.pop'), { lambda: true }), f('push', t('label.push'), { lambda: true })];
  }
  if (type === 'mealy') return [f('input', t('label.input'), { lambda: true }), f('output', t('label.output'), { lambda: true })];
  if (type === 'turing') {
    const moves = [['L', t('label.left')], ['R', t('label.right')], ['S', t('label.stay')]];
    return Array.from({ length: tapes }, (_, i) => {
      const suffix = tapes > 1 ? ` (${t('label.tape', { n: i + 1 })})` : '';
      return [
        f(`read${i}`, t('label.read') + suffix, { placeholder: '□', hint: i === 0 ? t('label.tm_hint') : undefined }),
        f(`write${i}`, t('label.write') + suffix, { placeholder: '□' }),
        { name: `move${i}`, label: t('label.move') + suffix, type: 'select', options: moves, value: initial[`move${i}`] || 'R' },
      ];
    }).flat();
  }
  return [f('text', t('label.read'), { lambda: true, placeholder: type === 'moore' ? '' : '[0-9]', hint: t('label.fa_hint') })];
}

/**
 * Asks for a transition label. `text` is the current label text (edit) or empty (new).
 * Resolves with the full label object, or null when cancelled.
 */
export async function askLabel({ api, type, tapes = 1, text = '', title }) {
  const initial = text ? fieldsFromText(type, text, tapes) : {};
  const result = await formDialog({
    title: title || t('label.title'),
    fields: formFields(type, tapes, initial),
    validate: async (values) => ({ label: await labelFromText(api, type, textFromFields(type, values, tapes), tapes) }),
  });
  return result ? result.label : null;
}
