// Pure side of the pumping lemma panel: the view model of an engine answer (boxes in JFLAP order, who must
// move, result, cases) and the helpers of the decomposition boxes.

/** Which box each thing the user can be asked for lives in. */
const BOX_OF = { m: 1, w: 2, decomposition: 3, i: 4 };
const INPUT_OF = { 1: 'number', 2: 'text', 3: 'decomposition', 4: 'number', 5: null };

/** `pumping_new` / `pumping_move` answer -> what the panel draws. `kind` is 'regular' or 'context_free'. */
export function pumpingView(response, kind) {
  const view = response.view;
  const result = response.result
    ? { userWins: response.result.user_wins, inLanguage: response.result.in_language, pumped: response.result.pumped, message: response.result.message, takeaway: response.result.takeaway }
    : null;
  const finished = (result !== null && result.userWins === true) || Boolean(response.conclusion);
  const waitingFor = BOX_OF[view.awaiting];
  const cases = view.cases
    ? {
        total: view.cases.total,
        rows: view.cases.rows,
        selected: view.cases.selected,
        message: view.cases.message,
        listEnabled: view.cases.list_enabled,
        addReplaceEnabled: view.cases.add_replace_enabled,
        temporary: view.cases.temp_decomposition,
      }
    : null;
  return {
    title: view.title,
    languageHtml: view.html_title,
    objective: view.objective,
    prompt: view.prompt,
    notice: view.notice,
    mode: view.mode,
    awaiting: view.awaiting,
    m: response.lemma ? response.lemma.m : -1,
    parts: kind === 'context_free' ? 4 : 2,
    boxes: view.boxes.map((box) => ({
      number: box.number,
      visible: box.visible,
      caption: box.caption,
      content: box.content,
      message: box.message,
      editable: !finished && box.number === waitingFor && box.visible,
      input: INPUT_OF[box.number],
    })),
    attempts: view.attempts,
    reply: response.reply ? { accepted: response.reply.accepted, message: response.reply.message } : null,
    result,
    conclusion: response.conclusion || null,
    finished,
    cases,
  };
}

/** Lengths of the pieces cut to fit `w`: each at least 0 and never past the end; `n` pieces when none are given. */
export function clampLengths(w, lengths, n = lengths.length || 2) {
  let left = [...w].length;
  const out = [];
  for (let i = 0; i < n; i++) {
    const wanted = Number.isFinite(lengths[i]) ? Math.trunc(lengths[i]) : 0;
    const length = Math.max(0, Math.min(left, wanted));
    out.push(length);
    left -= length;
  }
  return out;
}

/** `[x, y, z]` or `[u, v, x, y, z]` for the lengths of the pieces before the last one. */
export function decompositionParts(w, lengths) {
  const chars = [...w];
  const cut = clampLengths(w, lengths);
  const parts = [];
  let at = 0;
  for (const length of cut) {
    parts.push(chars.slice(at, at + length).join(''));
    at += length;
  }
  parts.push(chars.slice(at).join(''));
  return parts;
}

/** Cut points (where each piece ends) -> lengths; points out of order give an empty piece. */
export function lengthsFromIndices(indices) {
  let previous = 0;
  return indices.map((index) => {
    const length = Math.max(0, index - previous);
    previous = Math.max(previous, index);
    return length;
  });
}

/** The languages of one lemma ('regular' or 'context_free') from the `pumping_languages` answer. */
export function pumpingLanguages(catalogue, kind) {
  return catalogue.languages.filter((language) => language.kind === kind);
}

/** Cut points (where each piece ends) of the lengths; the inverse of `lengthsFromIndices`. */
export function indicesFromLengths(lengths) {
  let total = 0;
  return lengths.map((length) => (total += length));
}

/** Which catalogue entry the game on the table is: the longest title contained in the game's title. */
export function currentLanguage(title, languages) {
  let best = null;
  for (const language of languages) {
    if (title.includes(language.title) && (!best || language.title.length > best.title.length)) best = language;
  }
  return best;
}
