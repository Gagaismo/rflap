# JFLAP 7.1 `.jff` element names

Source: `file/xml/*Transducer.java` of the decompiled JFLAP 7.1, checked against files
JFLAP wrote itself (`tests/fixtures/jff/`). Names are case-sensitive. All text is raw
(JFLAP does not trim); a missing or empty element means the empty string unless noted.

## Document

```xml
<?xml version="1.0" encoding="UTF-8" standalone="no"?><!--Created with JFLAP 6.4.--><structure>
	<type>TYPE</type>
	...
</structure>
```

The comment says 6.4 even in 7.1. Indentation is one tab per level and elements without
content are `<x/>`. On Windows JFLAP's line separator is CRLF and shows up as `&#13;` at
the end of every line; both are accepted. The reader finds `<type>` as the first
descendant with that name (`getElementsByTagName("type").item(0)`); an unknown type is an
error in JFLAP.

## Type strings

| `<type>` | Structure | Transducer |
|---|---|---|
| `fa` | finite automaton | `FSATransducer` |
| `pda` | pushdown automaton | `PDATransducer` |
| `turing` | Turing machine (1-5 tapes) | `TMTransducer` |
| `mealy` | Mealy machine | `MealyTransducer` |
| `moore` | Moore machine | `MooreTransducer` |
| `grammar` | grammar | `GrammarTransducer` |
| `re` | regular expression | `RETransducer` |
| `lsystem` | L-system | `LSystemTransducer` |
| `regular pumping lemma` | regular pumping lemma game | `RegPumpingLemmaTransducer` |
| `context-free pumping lemma` | context-free pumping lemma game | `CFPumpingLemmaTransducer` |

(The pumping lemma types are those two strings with spaces, not `regpumping`/`cfpumping`.)

## Automata (`fa`, `pda`, `turing`, `mealy`, `moore`) — `AutomatonTransducer`

JFLAP 7 writes `<structure><type/><automaton>...</automaton></structure>`. When reading,
the first `<automaton>` descendant is used, or the `<structure>` itself if there is none
(older files). `turing` with more than one tape adds `<tapes>N</tapes>` right after
`<type>` (outside `<automaton>`); a missing `<tapes>` means 1.

Inside the automaton node, in this order when written:

- `<!--The list of states.-->` (only if there are states), then one element per state:
  - `<state id="N" name="NAME">` — `id` is required (an invalid one is a read error in
    this port; JFLAP maps it to -1); `name` defaults to `qN`; a `<name>` child overrides it.
  - children: `<x>`, `<y>` (Java double text, e.g. `50.0`; JFLAP rounds to int when it
    loads), `<label>` (optional), `<initial/>`, `<final/>`, and for `moore` `<output>`.
  - states are created in id order; a duplicate id is an error.
  - Turing machines write `<block id name>` instead of `<state>`, with `<tag>` (internal
    name), `<x>`, `<y>`, `<initial/>`, `<final/>`; blocks reference extra
    `<automaton>`-named elements (one per block name) listed after the transitions
    under the comment `The list of automata`. Reading also accepts `<state>` with
    `<tag>`. Transitions of a block carry the attribute `block="true"`.
- `<!--The list of transitions.-->` (only if any), then one `<transition>` each:
  - `<from>ID</from>`, `<to>ID</to>` (required; an unknown state is an error),
    `<controlx>`, `<controly>` (integers, only when the curve was moved),
    then the machine-specific children:
  - `fa`: `<read>` (empty = λ).
  - `pda`: `<read>`, `<pop>`, `<push>` (each empty = nothing).
  - `mealy`: `<read>`, `<transout>` (output). `moore`: same transition children as `mealy`
    (JFLAP writes `<transout>` for Moore transitions too) plus `<output>` on each state.
  - `turing`: per tape `<read tape="i">`, `<write tape="i">`, `<move tape="i">`;
    the `tape` attribute is written only when there is more than one tape and defaults
    to 1 when reading. Blank is written as empty (read and write); a missing `<move>`
    is `R`; moves are `L`, `R`, `S`. A block transition has only `<read>` plus
    `block="true"`.
- `<note>` elements (after transitions): `<text>`, `<x>`, `<y>`. A note without `<text>`
  is skipped when reading.

## `grammar` — `GrammarTransducer`

```xml
<!--The list of productions.-->
<production><left>S</left><right>aSb</right></production>
```

Every `<production>` descendant is read; `left`/`right` missing or empty are `""`
(an empty right side is λ). No other elements.

## `re` — `RETransducer`

```xml
<!--The regular expression.-->
<expression>(a+b)*</expression>
```

A missing `<expression>` is the empty expression.

## `lsystem` — `LSystemTransducer`

```xml
<!--The L-system axiom.-->
<axiom>F</axiom>            <!-- symbols separated by one space; required -->
<!--The rewriting rules.-->
<production><left>F</left><right>F + F</right><right>...</right></production>
<!--The drawing parameters.-->
<parameter><name>angle</name><value>90</value></parameter>
```

One `<production>` per left side with one `<right>` per replacement (each replacement is
a space-separated symbol list). `<parameter>` elements are name/value pairs (a missing
`<name>` skips the parameter).

## Pumping lemma games — `PumpingLemmaTransducer` and subclasses

Shared children of `<structure>` (after `<type>`): `<name>` (lemma title, e.g.
`L = {a^n b^n}`), `<first_player>`, `<m>`, `<w>`, `<i>`, then zero or more `<attempt>`.

- `regular pumping lemma`: also `<xLength>`, `<yLength>` (lengths of the decomposition).
- `context-free pumping lemma`: also `<uLength>`, `<vLength>`, `<xLength>`, `<yLength>`
  and zero or more `<case>` elements with `<caseULength>`, `<caseVLength>`,
  `<caseXLength>`, `<caseYLength>`, `<caseI>`.

Written order (regular): `name`, `first_player`, `m`, `w`, `i`, `xLength`, `yLength`,
`attempt`*. Context-free: `name`, `first_player`, `m`, `w`, `i`, `uLength`, `vLength`,
`xLength`, `yLength`, `attempt`*, `case`*. All of them are required when reading
(JFLAP throws if one is missing).
