// What a tool result's `structuredContent` says about the tab that shows it (pure, no DOM).

import { notationOf } from './tm-model.js';

/** Keys of the result that describe the tab and are not content for the Assistant panel. */
export const META_KEYS = ['tool', 'title', 'notation'];

/** The extras the Assistant panel shows. */
export const contentExtras = (extras) => Object.fromEntries(Object.entries(extras || {}).filter(([key]) => !META_KEYS.includes(key)));

/** The title of the machine (`title`), or null: the tab and the caption over the diagram use it. */
export function resultTitle(structured) {
  const title = structured && typeof structured.title === 'string' ? structured.title.trim() : '';
  return title || null;
}

/** 'catedra' | 'jflap' when the result asks for one, else null (the user's choice stays). */
export function resultNotation(structured) {
  const n = structured && structured.notation;
  return n === 'catedra' || n === 'jflap' ? notationOf(n) : null;
}
