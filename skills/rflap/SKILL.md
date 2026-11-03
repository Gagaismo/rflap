---
name: rflap
description: Solve exercises of "Sintaxis y Semántica de los Lenguajes" (finite automata, regular expressions, regular and context-free grammars, pushdown automata, Turing machines, FIRST/FOLLOW, LL(1) and SLR(1) tables, pumping lemma) by designing, verifying and drawing the answer with Rflap (JFLAP 7.1 in Rust), through its MCP tools (rflap_*) or its `rflap` CLI. Use when a student asks for an automaton or grammar for a language, a conversion (NFA to DFA, minimization, regex to automaton, grammar to automaton), a parse table, or a check that two descriptions are equivalent.
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
grammar of a PDA use `rflap_convert` with `to: "cfg"` (JFLAP's construction needs one final state and
transitions that pop one symbol and push none or two; otherwise the error says which transition
breaks it); for the reverse, `to: "pda_ll"` or `"pda_lr"`.

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

CLI: `rflap call '{"op":"parse_tables","grammar":"E -> TA\nA -> +TA | λ\nT -> FB\nB -> *FB | λ\nF -> (E) | i"}'`
(run `rflap --help` for the dedicated subcommands).

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

## Rules of thumb

* A tool error is a message with a code (`unknown_state`, `invalid_label`, ...): read it, fix the
  input, retry. Do not paste raw JSON at the student.
* Prefer `constraint_text` or `regex` over hand-drawing states; draw by hand (compact document +
  `rflap_show`) only when the exercise gives the automaton.
* Minimal means minimal: if the student's answer has more states, run `rflap_convert` with
  `minimal_dfa` and show both.
* If no Rflap tool is available, say so and reason by hand; do not invent output.
