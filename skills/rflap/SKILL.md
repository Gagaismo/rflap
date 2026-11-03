---
name: rflap
description: Solve exercises of "Sintaxis y Semántica de los Lenguajes" (finite automata, regular expressions, regular and context-free grammars, pushdown automata, Turing machines, FIRST/FOLLOW, LL(1) and SLR(1) tables, pumping lemma) by designing, verifying and drawing the answer with Rflap (JFLAP 7.1 in Rust), through its MCP tools (rflap_*) or its `rflap` CLI. Use when a student asks for an automaton or grammar for a language, a conversion (NFA to DFA, minimization, regex to automaton, grammar to automaton), a parse table, or a check that two descriptions are equivalent. Also use it for Turing machine (MT) and linear bounded automaton (ALL) exercises in the notation of the course bibliography (definition ⟨Q, Σ_E, Σ_A, q0, F, f⟩, transition table, configuration sequences with ├─, ALL check, MT composition, membership by the sets Tk).
---

# Solving SSL exercises with Rflap

You are the reasoning; Rflap builds, minimizes, draws and **verifies**. Never hand a student an
automaton that Rflap has not verified against examples.

## Workflow

1. **Reason first.** Say what the machine must remember about the prefix read so far. Fix the
   alphabet. Decide whether the language is regular (automaton, regex, regular grammar), context
   free (grammar, PDA) or neither of those.
2. **Write a formal spec** and 4-8 accept and 4-8 reject examples. Always include the empty
   string and the boundary cases (shortest accepted word, longest rejected near-miss).
3. **Design.** Call `rflap_design_automaton` (MCP) or `rflap design` (CLI) with the spec.
4. **Read the verification.** If an example fails, either the spec or the example is wrong:
   decide which, fix it, design again. Do not explain a failed design as if it were right.
5. **Show it.** Hosts with MCP Apps render the widget by themselves; in a terminal, show the
   transition table from the reply and offer the viewer link if there is one.
6. **Explain** in the student's language: what each state remembers, why each transition goes
   where it goes, what the minimal DFA proves. Offer `rflap_simulate` / `rflap_step` to trace a
   word, and `rflap_equivalent` to compare with the student's own answer.

Documents are written in compact form: `{"type":"fa","initial":"q0","finals":["q2"],"transitions":[["q0","a","q1"],["q1","b","q2"]]}`;
`""` or `λ` is the empty string. A grammar is plain text, one production per line
(`S -> aSb | λ`, uppercase = variable). `rflap_capabilities` prints the syntax of every type.

## Constraint syntax (regular languages)

`starts_with("ab")`, `ends_with("ab")`, `contains("aa")`, `length >= 3`, `length % 2 == 0`,
`count(a) % 2 == 0`, `count(a,b) >= 2`, `regex("(a+b)*abb")`, combined with `and`, `or`, `xor`,
`not` and parentheses. `count(a) == count(b)` is accepted but is **not regular**: use a template.
Regex syntax is JFLAP's: `+` is union, `*` star, juxtaposition concatenation, `λ` the empty string.

## Example 1: strings over {a,b} ending in `ab`

Reasoning: after reading a prefix, only the longest suffix that is a prefix of `ab` matters:
nothing, `a`, or `ab`. Three states.

```json
{"alphabet": "ab", "constraint_text": "ends_with(\"ab\")",
 "examples": {"accept": ["ab", "aab", "bab", "abab"], "reject": ["", "a", "b", "ba", "abb", "aba"]}}
```

Tool: `rflap_design_automaton`. Expect a 3-state DFA, verification green. CLI equivalent:

```bash
rflap design --constraint 'ends_with("ab")' --alphabet ab -o ends_ab.jff
rflap run ends_ab.jff aab abb        # exit code 1 when some input is rejected
rflap render ends_ab.jff -o ends_ab.svg
```

## Example 2: a^n b^n (not regular)

Reasoning: the machine must count the a's, unbounded memory, so no finite automaton exists
(pumping lemma: `rflap_pumping` with `action: "new"` plays the proof). A PDA does it.

```json
{"alphabet": "ab", "template": {"name": "an_bn", "a": "a", "b": "b", "min": 0},
 "examples": {"accept": ["", "ab", "aabb", "aaabbb"], "reject": ["a", "b", "ba", "aab", "abb", "abab"]}}
```

Tool: `rflap_design_automaton` returns a PDA document. Other templates: `an_bm` (n <= m, n >= m, n != m),
`equal_count`, `balanced_brackets`, `wcwr`, `even_palindrome`, `palindrome`. To get the context-free
grammar of a PDA use `rflap_convert` with `to: "cfg"`. JFLAP's construction needs one final state
reached on an empty stack and transitions that pop one symbol and push none or two; Rflap rewrites
a PDA that is not in that form first (the `pda_to_jflap_form` entry of `steps` says so, and
`to: "pda_jflap_form"` does only that rewriting). For the reverse, `to: "pda_ll"` or `"pda_lr"`.

## Example 3: grammar to LL(1) table

```json
{"grammar": "E -> TA\nA -> +TA | λ\nT -> FB\nB -> *FB | λ\nF -> (E) | i"}
```

1. `rflap_grammar` (default action `type`) classifies it; `rflap_parse_tables` returns FIRST,
   FOLLOW, the LL(1) table (conflicts marked) and the SLR(1) tables.
2. If a cell has two productions, the grammar is not LL(1): explain the conflict (left recursion,
   common prefix), rewrite the grammar by hand and run `rflap_parse_tables` again.
3. `rflap_parse` with `parser: "ll1"` and `inputs: ["i+i*i", "i+"]` shows the accepted derivation
   and the rejected word.

CLI: `rflap tables --grammar 'E -> TA\nA -> +TA | λ\nT -> FB\nB -> *FB | λ\nF -> (E) | i'`, then
`rflap parse --grammar '...' 'i+i*i' --parser ll1`. `rflap --help` lists every subcommand and
`rflap call` sends any raw API request.

## Turing machines and ALL in the notation of the course

For MT / ALL exercises Rflap speaks the notation of the bibliography (CLASE 39 and 41), not JFLAP's.
Answer in Spanish with the course's terms.

1. **Reason first**: what does the machine do to the tape (mark, run to the end, come back)? MT, or
   ALL (the tape is `⟨w⟩`, the head starts on the first symbol of `w`)?
2. **Write the machine** as a compact `turing` document with labels `(e,s,m)`: move `I` left, `N`
   stay, `D` right, `Δ` the blank, several labels of one edge joined with `" / "`
   (`"(a,a,D) / (b,b,D)"`). List `"states"` with the names the student expects (1, 2, … or I, A, B, …);
   `"input_alphabet": "ab"` declares Σ_E (otherwise it is inferred). JFLAP labels (`a;b,R`) also work.
3. **Run Rflap** (MCP `rflap_mt_catedra` with `action`, or the CLI):

   | Need | MCP `action` | CLI |
   |---|---|---|
   | `MT = ⟨Q, Σ_E, Σ_A, q0, F, f⟩` and its sets | `definition` | `rflap catedra definition m.json` |
   | Transition table (Estado Actual \| Símbolo Entrada \| Estado Nuevo \| Símbolo Salida \| Movimiento Cabezal) | `table` | `rflap catedra table m.json --markdown` |
   | Configuration sequences `Δ1abaΔ ├─ …` | `run` with `inputs` | `rflap catedra run m.json aba ab` (`--all` for an ALL) |
   | Is it a valid ALL? | `all_check` with `inputs` | `rflap catedra all-check m.json ab b` |
   | MT1 followed by MT2 | `rflap_compose_tm` | `rflap catedra compose a.json b.json -o ab.json` |
   | Is `w` in the language of a non-contracting grammar? | `rflap_membership` | `rflap membership --grammar '…' abbc` |

   Run `run` on words the machine accepts **and** rejects (empty word included). For an ALL,
   **always** run `all_check` (static rules + dynamic run on sample words) and fix every violation
   before presenting it.
4. **Present**: the 6-tuple, the table, the graph (the widget, or `rflap render m.json -o m.svg`) and
   the sequences, each ending in `final/acepta` or `bloqueo/rechazo`.
5. **Remember**: final states halt (`f` is defined on Q-F, so a final state has no outgoing
   transitions); a missing transition is a *bloqueo*, which rejects; an MT may never halt
   (`no se detiene`), an ALL always accepts or rejects. Multitape uses JFLAP notation
   (`a;a,R|_;x,S`); `rflap_analyze` says whether it is multipista (all heads move together).
6. **ALL details**:
   - The tape may **shrink** (default): writing `⟨` over another symbol moving `D`, or `⟩` moving `I`
     (the course's `(a,⟨,D)` / `(c,⟩,I)`), is valid and comes back as a note
     (`ℹ️ q0 escribe ⟨ sobre a: la cinta se achica`), never a violation. The tape shows its real
     cells (`⟨q0aab⟩ ├─ ⟨⟨q1ab⟩ ├─ …`). For a stricter definition (⟨ ⟩ only written over themselves)
     pass `allow_shrink: false` (CLI `--no-shrink`); `rflap_mt_catedra`, `rflap_simulate`,
     `rflap_step` and `rflap_show` take it. Still violations: `⟨` written moving `I`/`N`, `⟩`
     written moving `D`/`N`, a delimiter read and not rewritten, the head crossing a delimiter.
   - Reading `⟨`/`⟩` and writing it back with `N` into a **final** state is valid (final states
     halt), with a note.
   - An input that already has its delimiters (`⟨aba⟩` or `<aba>`) is run without them, with a note.
   - Always write the delimiters as `⟨` `⟩`. A Turing machine without them whose labels use both
     `<` and `>` is normalized to `⟨` `⟩` (documents, `.jff`, tables); with only one of them, it
     stays an ordinary symbol.
   - `rflap_save_jff` / `rflap_open_jff` `path` accepts `~`, `%USERPROFILE%` and falls back from
     `~/Desktop` to `~/OneDrive/Desktop` (or `Escritorio`); the reply names the real path.

### Worked example 1: the palindrome MT (CLASE 39)

```json
{"type": "turing", "states": ["1","2","3","4","5","6","7","8"], "initial": "1", "finals": ["8"],
 "transitions": [
   ["1","(a,Δ,D)","2"], ["1","(b,Δ,D)","5"], ["1","(Δ,Δ,D)","8"],
   ["2","(a,a,D) / (b,b,D)","2"], ["2","(Δ,Δ,I)","3"],
   ["3","(a,Δ,I)","4"], ["3","(Δ,Δ,D)","8"],
   ["4","(a,a,I) / (b,b,I)","4"], ["4","(Δ,Δ,D)","1"],
   ["5","(a,a,D) / (b,b,D)","5"], ["5","(Δ,Δ,I)","6"],
   ["6","(b,Δ,I)","7"], ["6","(Δ,Δ,D)","8"],
   ["7","(a,a,I) / (b,b,I)","7"], ["7","(Δ,Δ,D)","1"]]}
```

`rflap_mt_catedra` with `action: "run"`, `inputs: ["aba", "ab", "bb"]` returns:

```text
Δ1abaΔ ├─ ΔΔ2baΔ ├─ ΔΔb2aΔ ├─ ΔΔba2Δ ├─ ΔΔb3aΔ ├─ ΔΔ4bΔΔ ├─ Δ4ΔbΔΔ ├─ ΔΔ1bΔΔ ├─ ΔΔΔ5ΔΔ ├─ ΔΔ6ΔΔΔ ├─ ΔΔΔ8ΔΔ final/acepta
Δ1abΔ ├─ ΔΔ2bΔ ├─ ΔΔb2Δ ├─ ΔΔ3bΔ bloqueo/rechazo
Δ1bbΔ ├─ ΔΔ5bΔ ├─ ΔΔb5Δ ├─ ΔΔ6bΔ ├─ Δ7ΔΔΔ ├─ ΔΔ1ΔΔ ├─ ΔΔΔ8Δ final/acepta
```

`definition` gives `Q = {1, …, 8}`, `Σ_E = {a, b} (inferido)`, `Σ_A = {Δ}`, `q0 = 1`, `F = {8}`.
(For the empty word the bibliography writes `Δ1Δ ├─ ΔΔ8`; Rflap prints `Δ1Δ ├─ ΔΔ8Δ`, a known deviation.)

### Worked example 2: the ALL "IGUAL" (same number of a and b)

```json
{"type": "turing", "states": ["I","A","B","C","D","F"], "initial": "I", "finals": ["F"],
 "transitions": [
   ["I","a,X,I","A"], ["I","(b,b,D) / (X,X,D)","I"], ["I","(⟩,⟩,I)","D"],
   ["A","(b,b,I) / (X,X,I)","A"], ["A","(⟨,⟨,D)","B"],
   ["B","(a,a,D)","B"], ["B","(b,X,I)","C"], ["B","(X,X,D)","B"],
   ["C","(a,a,I) / (X,X,I)","C"], ["C","(⟨,⟨,D)","I"],
   ["D","(X,X,I)","D"], ["D","(⟨,⟨,D)","F"]]}
```

`action: "all_check"`, `inputs: ["ab", "b"]` reports that it follows the ALL rules and shows:

```text
⟨Iab⟩ ├─ A⟨Xb⟩ ├─ ⟨BXb⟩ ├─ ⟨XBb⟩ ├─ ⟨CXX⟩ ├─ C⟨XX⟩ ├─ ⟨IXX⟩ ├─ ⟨XIX⟩ ├─ ⟨XXI⟩ ├─ ⟨XDX⟩ ├─ ⟨DXX⟩ ├─ D⟨XX⟩ ├─ ⟨FXX⟩ final/acepta
⟨Ib⟩ ├─ ⟨bI⟩ ├─ ⟨Db⟩ bloqueo/rechaza
```

A transition such as `["I","(⟩,X,I)","D"]` is reported as a violation: `La transición I --(⟩,X,I)--> D
lee ⟩: debe escribir ⟩ y moverse a la izquierda (I).` Student-facing reference: `docs/catedra.md`.

## Other frequent requests

| Request | Tool |
|---|---|
| NFA to DFA, minimize, add the trap state | `rflap_convert` with `to: "dfa"`, `"minimal_dfa"`, `"complete"` |
| Regular expression to automaton, and back | `rflap_convert` with `to: "nfa"` (from regex), `to: "regex"` (from fa) |
| Regular grammar to automaton, and back | `rflap_convert` with `to: "fa"`, `to: "regular_grammar"` |
| Remove λ / unit / useless productions, Chomsky normal form | `rflap_grammar` with `action` `remove_lambda`, `remove_unit`, `remove_useless`, `cnf` |
| Is the student's automaton right? | `rflap_equivalent` against a regex or a reference automaton; it returns a counterexample |
| Trace a run | `rflap_simulate` (whole trace, or `inputs` for a table), `rflap_step` (one step at a time) |
| Open or save a JFLAP file | `rflap_open_jff`, `rflap_save_jff` |
| Turing machine / ALL exercise in the course notation | `rflap_mt_catedra` (see the section above); `rflap_simulate`, `rflap_step` and `rflap_show` also run a machine as an ALL with `model: "all"` (or when its labels use ⟨ ⟩) and take `notation: "catedra" \| "jflap"` |
| Name the drawing, place the states | the compact document takes `title` (alias `name`), `layout` (default `layered`: initial state at the left) and `points` (`rflap_capabilities` with `section: "layout"`) |
| Turing machine run with its final tape | `rflap_simulate` (`tape_outputs` is the final content of each tape, blanks trimmed; `acceptance`: `final_state`, `halting`, `both`) |
| Anything without a dedicated tool (layout, user-controlled parsing, ...) | `rflap_call` with `{"request": {"op": ...}}`; the answer is `{"ok": ..., "result" or "error"}` |

## Rules of thumb

* A tool error is a message with a code (`unknown_state`, `invalid_label`, ...): read it, fix the
  input, retry. Do not paste raw JSON at the student.
* Prefer `constraint_text` or `regex` over hand-drawing states; draw by hand (compact document +
  `rflap_show`) only when the exercise gives the automaton.
* Minimal means minimal: if the student's answer has more states, run `rflap_convert` with
  `minimal_dfa` and show both.
* If no Rflap tool is available, say so and reason by hand; do not invent output.
