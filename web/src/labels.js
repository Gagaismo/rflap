// The label form of a transition, per automaton type (JFLAP asks the same fields):
//   fa / moore : one field (the string read) with a λ button
//   pda        : input, pop, push
//   turing     : read, write and move for each tape (cátedra notation: (e,s,m) with Δ and I / N / D for one tape;
//                JFLAP notation: e;s,m with □ and L / S / R); a typed `(a,X,D)` is accepted in a field of its own
//   mealy      : input, output
// The form builds the compact label text; the engine parses it (`labelFromText`), so a wrong label is
// reported with the engine's own message and the dialog stays open.

import { labelFromText } from './doc.js';
import { formDialog } from './dialogs.js';
import { t } from './i18n.js';
import { blankOf, formValues, moveLetters, textFromForm } from './tm-model.js';

const unlambda = (s) => (s === 'λ' || s === 'ε' ? '' : s);

/** Text -> field values of the form. `notation` is how a Turing machine shows its symbols and moves. */
export function fieldsFromText(type, text, tapes = 1, notation = 'jflap') {
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
    Object.assign(values, formValues(text, notation, tapes));
  } else {
    values.text = unlambda(text);
  }
  return values;
}

/** Field values -> label text. */
export function textFromFields(type, values, tapes = 1, notation = 'jflap') {
  const lam = (s) => (s && s.trim() ? s.trim() : 'λ');
  if (type === 'pda') return `${lam(values.input)},${lam(values.pop)};${lam(values.push)}`;
  if (type === 'mealy') return `${lam(values.input)};${lam(values.output)}`;
  if (type === 'turing') return textFromForm(values, notation, tapes);
  return (values.text || '').trim();
}

function formFields(type, tapes, initial, notation) {
  const f = (name, label, extra = {}) => ({ name, label, value: initial[name] ?? '', ...extra });
  if (type === 'pda') {
    return [f('input', t('label.input'), { lambda: true }), f('pop', t('label.pop'), { lambda: true }), f('push', t('label.push'), { lambda: true })];
  }
  if (type === 'mealy') return [f('input', t('label.input'), { lambda: true }), f('output', t('label.output'), { lambda: true })];
  if (type === 'turing') {
    const blank = blankOf(notation);
    const names = { L: t('label.left'), S: t('label.stay'), R: t('label.right'), I: t('label.left'), N: t('label.stay'), D: t('label.right') };
    const moves = moveLetters(notation, tapes).map((letter) => [letter, `${letter} — ${names[letter]}`]);
    const fields = Array.from({ length: tapes }, (_, i) => {
      const suffix = tapes > 1 ? ` (${t('label.tape', { n: i + 1 })})` : '';
      return [
        f(`read${i}`, t('label.read') + suffix, { placeholder: blank, insert: blank, hint: i === 0 ? t('label.tm_hint') : undefined }),
        f(`write${i}`, t('label.write') + suffix, { placeholder: blank, insert: blank }),
        { name: `move${i}`, label: t('label.move') + suffix, type: 'select', options: moves, value: initial[`move${i}`] || moves[moves.length - 1][0] },
      ];
    }).flat();
    const typed = notation === 'catedra' && tapes === 1 ? '(a,X,D)' : 'a;X,R';
    return [...fields, f('full', t('label.typed'), { placeholder: typed, hint: t('label.typed_hint', { example: typed }) })];
  }
  return [f('text', t('label.read'), { lambda: true, placeholder: type === 'moore' ? '' : '[0-9]', hint: t('label.fa_hint') })];
}

/**
 * Asks for a transition label. `text` is the current label text (edit) or empty (new).
 * Resolves with the full label object, or null when cancelled.
 */
export async function askLabel({ api, type, tapes = 1, text = '', title, notation = 'catedra' }) {
  const initial = text ? fieldsFromText(type, text, tapes, notation) : {};
  const result = await formDialog({
    title: title || t('label.title'),
    fields: formFields(type, tapes, initial, notation),
    validate: async (values) => ({ label: await labelFromText(api, type, textFromFields(type, values, tapes, notation), tapes) }),
  });
  return result ? result.label : null;
}
