# Differences from JFLAP 7.1

Rflap ports the algorithms of JFLAP 7.1 and checks them against the real jar (see
[testing.md](testing.md)): where a deviation exists it is intentional and listed here, grouped by
module. Anything not listed behaves like JFLAP, including its messages, its order of steps and its
quirks. Sources: the plan log (`odd/tasks/rflap-port.md`, L5 and L8-L10), the module documentation of
`crates/rflap-core/src/**`, `crates/rflap-core/src/jff/FORMAT.md` and the tests that pin each point.

Legend: **deviation** = Rflap does something else on purpose; **addition** = something JFLAP does not
have; **JFLAP bug** = JFLAP misbehaves and Rflap does not copy it; **same quirk** = a JFLAP oddity that
is kept so results agree.

## Finite automata (`fa`, `analysis`, `sim`)

* **Deviation: ranges and λ-closure.** In the simulator with closure (`mode: closure`), a transition
  with a range label `[x-y]` also takes the λ-closure of its destination. JFLAP does not: with a range
  into a state that has a λ transition out, JFLAP stops one state early. This is a **JFLAP bug** that
  the port fixes (plan log L5).
* **Deviation: what a range is.** A range is recognized only in the exact form `[x-y]`, as the whole
  label (L5).
* **Same quirk:** every other simulator behaviour (step by state, with closure, fast run) is
  JFLAP's; a Turing machine simulator is deterministic and follows the first applicable transition,
  as in JFLAP.
* **Addition:** `combine` and `remove_useless_states` also work on Turing machines; a combined Turing
  machine keeps the building blocks of both machines (they need the same number of tapes). Removing
  useless states keeps only states that can reach a final state, so it changes a machine that
  accepts by halting.

## Finite automaton conversions (`fa_convert`)

* **Deviation: equivalence.** JFLAP (`FSAEqualityChecker`) converts both automata to DFAs, minimizes
  them and checks that they are isomorphic. Rflap searches the product of the two DFAs breadth-first.
  The verdict is the same, and the search yields a **shortest counterexample**, which JFLAP does not
  report.
* NFA to DFA, minimization (distinguishability tree), FA to regular expression (state elimination)
  and regular expression to NFA follow JFLAP's constructions step by step; the steps are returned as
  data so a UI can replay them.

## Pushdown automata and conversions (`pda`, `convert`)

* **Addition: `pda_jflap_form`.** JFLAP refuses to convert a PDA to a grammar unless it has exactly
  one final state, its transitions into the final state pop only `Z`, and every transition pops one
  symbol and pushes none or two. Rflap adds a conversion that rewrites any PDA accepting by final
  state into that form (a new bottom marker, intermediate states, one drain state), with steps;
  `convert cfg` applies it automatically when the input is not in form and reports it in `steps`
  (the first step has `kind: "pda_to_jflap_form"`). A PDA already in form goes through unchanged. The
  preconditions JFLAP checks (`check_pda_form`) and their messages are unchanged.
* **Deviation: order of terminals.** The CFG to PDA conversions (LL and LR) take terminals in
  character order; JFLAP iterates a hash set, whose order is not specified.
* **Same quirk:** the PDA to CFG conversion ends with JFLAP's purge of the productions that cannot
  derive a string or cannot be reached; variables are named in JFLAP's order (`A`, `B`, ... skipping
  `S`, then the following code points).
* A PDA starts with `Z` on the stack and accepts by final state or by empty stack, as JFLAP asks.

## Turing machines (`turing`, `convert::tm_grammar`)

* **Addition: `tape_outputs`.** `simulate` returns the final content of every tape, blanks around the
  data trimmed (`aa` for a tape that ends as `aa□`). JFLAP's own "output" of a tape is only what lies
  from the head up to the next blank, which is empty when the head ends past the data;
  `rflap_core::turing::tape_outputs` keeps that meaning; the new `tape_contents` is what the API
  returns under the name `tape_outputs`.
* **Deviation: TM to grammar.** Productions keep the order in which they are generated (template
  first, then each transition) instead of JFLAP's sort, and symbol sets are iterated in order of
  first use. The limits JFLAP shares are reported in `warnings`: transitions that stay (`S`) produce
  nothing, the empty input cannot be derived, and a move only fires next to cells whose current symbol
  some transition writes. Only one-tape machines without building blocks convert.
* **Same quirk:** blocks (building blocks) run when entered; acceptance by final state is the default.

## Mealy and Moore machines (`mealy`)

* Same as JFLAP. A Moore `<transout>` in a `.jff` is the output of the destination state; see the
  `.jff` section.

## Grammars and parsers (`grammar`)

* **Same quirk** (kept on purpose, verified against the jar for λ, unit and useless removal, CNF,
  FIRST/FOLLOW, LL(1) and SLR(1)): variables are uppercase characters, the start variable is the left
  side of the first production, the brute-force parser searches breadth-first with JFLAP's pruning,
  SLR(1) numbers states and productions as JFLAP does (`r1` is the first user production), cell text
  is `s3`, `r4`, `acc`.
* **Deviation: budgets.** A brute-force search takes a `budget` of candidate forms and answers
  `undecided` when it runs out (JFLAP keeps asking the user whether to go on); the CLI reports it
  with exit code 3. Runs of machines take `max_configs` in the same way.
* **Deviation: errors per input.** A parser that cannot handle a grammar (LL(1) on a grammar that is
  not LL(1), CYK outside Chomsky normal form, a symbol outside the grammar) reports it in the result
  of that input (`outcome: "error"`) instead of opening a dialog.

## Layouts (`layout`)

* **Addition: overlap removal.** After GEM, Random and Two Circle, a final pass pushes states apart
  until none is closer than `min_distance` (JFLAP guarantees no minimum distance for those three). `LayoutOptions.min_distance = 0` turns it off.
* **Deviation: randomness.** `random` and `gem` use a small seeded PRNG (`LayoutOptions.seed`), so a
  layout is reproducible: the same document and seed give the same points.
* **Same quirk:** the tree layout, like JFLAP, skips the element after a removal when joining chains
  with two or more inner neighbours.
* The algorithm names are `circle`, `gem`, `random`, `spiral`, `tree`, `tree-degree`, `two-circle`
  everywhere (serde, `FromStr`, CLI, API); `tree-by-degree` is still accepted on input.

## L-systems (`lsystem`)

* **Same results as JFLAP**, verified against the jar: expansions, image sizes, final bounds, colours
  and `.jff` files byte for byte (plan log L8). The turtle repeats JFLAP's matrix arithmetic in the
  same order; `sin`/`cos` may differ in the last bit. Seeded stochastic rules use a `java.util.Random`
  with JFLAP's draw order.
* **Deviation: errors.** An expression that makes JFLAP throw (`Turtle.valueOf`) is an error value
  here, not an exception.
* **Deviation: rendering target.** Drawing produces SVG (and the same shapes for the web editor); JFLAP
  paints on a Swing canvas.

## Pumping lemma (`pumping`)

* **Same results as JFLAP** for the 26 predefined languages: titles, ranges, cases, the choices of
  `m`, `w`, the decomposition and `i` for a seeded `java.util.Random`, and membership of every short
  string (plan log L8). Moves and messages are JFLAP's.
* **Deviation: Java exceptions.** Where JFLAP throws (`StringIndexOutOfBoundsException` when a
  decomposition runs past the end of `w`, `NullPointerException` when `w` does not have the expected
  shape, `IllegalArgumentException` in `fetchRandInt(lo, hi)` with `hi < lo`), Rflap does nothing or
  returns `lo`, so a game never crashes. The games are serializable between calls.

## `.jff` files (`jff`)

Reading and writing of every type is checked against files written by JFLAP 7.1 itself, and files
written by Rflap load in JFLAP (plan log L8).

* **JFLAP bug: building-block tags.** JFLAP names the machine inside a block with a tag (`Machine<id>`
  by default) and resolves a tag to the first element of the document with that name. Two blocks with
  the same default tag at different levels make JFLAP 7.1 recurse until `StackOverflowError` when it
  reopens **its own** file. Rflap writes unique tags (`Block<id>`, and `<tag>_<id>` one level down)
  and reads both.
* **Deviation: unknown tags.** An unknown block tag is an error in Rflap; JFLAP silently makes an
  empty machine.
* **Deviation: Turing machine transitions.** Rflap requires every tape of the machine to appear in a
  transition and rejects a `tape` attribute beyond the machine's (`TapeCount`, `TapeOutOfRange`);
  JFLAP fills missing tapes with blank/blank/R.
* **Deviation: state labels.** Rflap writes a Turing state's `<label>` after `<y>`, which JFLAP reads;
  JFLAP does not write it.
* **Deviation: Moore outputs.** JFLAP copies the `<transout>` of a transition into its destination
  state when it loads, so an empty `<transout>` loses the destination's `<output>`. Rflap writes the
  transition output and reads it only when present.
* **Deviation: state ids.** An invalid state `id` is a read error (JFLAP maps it to -1).
* **Same quirk:** the file comment says `Created with JFLAP 6.4.` even in 7.1; indentation is one tab,
  CRLF line ends and `&#13;` are accepted, element names are case-sensitive. The full list is in
  `crates/rflap-core/src/jff/FORMAT.md`.

## Compatibility with classic JFLAP (`jflap_check`)

Every machine Rflap designs, shows or saves must open in JFLAP 7.1 with the same states, initial
state, finals, labels and symbols, and run with the same verdicts (S8 of the plan `odd/tasks/all-shrink.md`).
`jflap_check` (API op; the MCP tools run it and put the lines `⚠️ JFLAP 7.1: …` / `ℹ️ JFLAP 7.1: …` in
the reply and the result in `structuredContent.jflap`) reports `{compatible, issues, notes}`. What was
found by comparing against the jar (`crates/rflap-api/tests/jflap_diff.rs`, harness `HarnessSim.java`):

| Finding | Kind | Evidence |
|---|---|---|
| More than 5 tapes | issue `too_many_tapes` | JFLAP's loader throws `DataException: 6 invalid # of tapes. Valid # of tapes 1-5` |
| Range `[x-y]` into a state with a λ exit | issue `range_before_lambda` | JFLAP's Run (`SimulatorFactory` gives `FSAStepWithClosureSimulator`) rejects `5` for `q0 -[0-9]-> q1 -λ-> q2`; Rflap accepts (the known closure bug, see above) |
| ALL (⟨ ⟩) | note `all_model` | JFLAP has no ALL model: no delimiters are added and the head starts on the first symbol typed; the note says how to run it (see `catedra.md`) |
| Declared Σ_E | note `input_alphabet_not_saved` | the `.jff` has no field for it |
| Turing machine without finals | note `accepts_by_halting` | JFLAP accepts by final state unless "Accept by Halting" is ticked |
| PDA without finals | note `accepts_by_empty_stack` | JFLAP asks "Accept by" at run time; the file does not store it |

Everything else compared **equal**: states, names, labels, initial and finals; FA with λ, multi-character
labels and ranges; PDA with multi-symbol pushes; Mealy and Moore outputs; one- and two-tape Turing machines
(blank `Δ` written as the empty string, `N` as `S`, several labels on one edge as several transitions,
nondeterministic machines run on the first applicable transition in both); the four ALL fixtures of the
course load with the same graph.

## Drawing (`render`)

* **Deviation: SVG instead of Swing.** The look follows `AutomatonDrawer`/`StateDrawer`/`CurvedArrow`
  (same geometry), drawn as SVG, Markdown tables, Graphviz DOT and text so that chats can show it.
  JFLAP uses the real `FontMetrics` for label sizes; Rflap uses an approximate ascent.

## Additions that JFLAP does not have

| Addition | Where |
|---|---|
| Design engine: language (constraint, regex, regular grammar or non-regular template) to a minimal, named, drawn and verified automaton | `rflap-core::design`, `rflap design`, `rflap_design_automaton` |
| One JSON API for every operation | `rflap-api` |
| MCP server, widget for MCP Apps and ChatGPT, agent skill | `rflap-mcp`, `web/`, `skills/` |
| Shortest counterexample in equivalence | `equiv`, `rflap_equivalent` |
| `pda_jflap_form`, automatic in `convert cfg` | see above |
| `tape_outputs` with the final tape | see above |
| `combine`, `remove_useless_states` for Turing machines | see above |
| Step data of every conversion (`--steps`) for replay | all conversions |
| Plain-language explanation of a document (`explain`) | `rflap explain` |
| JFLAP 7.1 compatibility check (`jflap_check`), run by the MCP tools | `rflap call '{"op":"jflap_check",...}'`, `rflap_*` replies |

## JFLAP bugs found while porting

| Bug | Effect | What Rflap does |
|---|---|---|
| Building-block tag collision | `StackOverflowError` reopening a JFLAP-written Turing machine with nested blocks that use default tags | writes unique tags |
| Range transitions in the closure simulator ignore λ-closure of the destination | a range into a state with outgoing λ transitions rejects strings it should accept | takes the closure |
| Moore `<transout>` empty on load | loses the output of the destination state | see `.jff` section |
| Exceptions in a few pumping-lemma helper methods | a game crashes on some inputs | no-op or `lo`, see above |
| Hash-set iteration order in two conversions | the order of terminals (CFG to PDA) and of symbol sets (TM to grammar) is unspecified | character order, order of first use |
