# rflap-cli

The `rflap` command: JFLAP 7.1 in Rust for terminals and AI agents (Claude Code, Codex, ...).
It is a thin layer over [`rflap-api`](../rflap-api): every subcommand builds an API request,
calls it and prints the answer as clear text, or as the raw JSON answer with `--json`. No
algorithm lives in this crate.

```text
cargo install --path crates/rflap-cli     # installs `rflap`
rflap capabilities                        # what it can do
rflap design --constraint 'ends_with("ab")' --alphabet ab -o ends_ab.jff
rflap run ends_ab.jff aab abb             # exit 1: abb is rejected
```

## Files and inline documents

| Where a FILE is expected | Meaning |
|---|---|
| `x.jff` | a JFLAP file of any type (read with `open_jff`) |
| `x.json` | a compact or full Rflap document (an API answer saved with `--json` also works) |
| `x.txt`, `x.grammar` | a grammar, one production per line |
| `-` | standard input; the format is sniffed (`<` is `.jff`, `{` is JSON, else a grammar) |
| `regex:(a+b)*abb`, `grammar:S -> aSb` | an inline document (`equiv`, `combine`, ...) |

Commands that take one document also have `--regex EXPR` and `--grammar TEXT|FILE`. A literal
`\n` inside inline grammar text separates productions. `λ` (or `ε`) as an input means the
empty string (an empty shell argument is easy to lose, especially in PowerShell).

Output files (`-o`): `.jff` is saved as a JFLAP file, `.json` as the normalized document and
`.svg` as a drawing.

## Exit codes

| code | meaning |
|---|---|
| 0 | success; every input was accepted |
| 1 | some input was rejected, or the automata are not equivalent (`design` with a failing example too) |
| 2 | usage or input error; stderr has `error: <code>: <message>` (the codes are those of the API plus `io_error` and `invalid_argument`) |
| 3 | undecided: a configuration or search budget ran out (`--max-configs`, `--budget`) |

When several inputs give different outcomes the highest-priority code wins:
2, then 3, then 1, then 0.

With `--json` the raw API answer goes to stdout (an error as `{"error": {...}, "ok": false}`),
with the same exit codes. `run` with several inputs sends one `multiple_run` request; with
`--trace` it sends one `simulate` per input and `--json` prints an array of answers.

## Commands

| command | what it does |
|---|---|
| `call [JSON\|-]` | raw API passthrough (always prints JSON) |
| `info FILE` | type, states, alphabet, determinism, λ transitions, unreachable and useless states |
| `run FILE INPUT...` | run inputs (`--mode step\|closure`, `--acceptance final\|empty-stack\|halting\|both`, `--trace`, `--max-configs N`, `--inputs-file F`, `--tape T` per tape) |
| `step FILE INPUT --steps N` | the configurations reached after N steps |
| `convert FILE --to TARGET [-o OUT] [--steps]` | every conversion of the API (`rflap convert --help` lists the targets) |
| `equiv A B` | language equivalence of two finite automata, regexes or regular grammars |
| `combine A B -o OUT` | append B below A |
| `render FILE [--format svg\|md\|dot\|text] [-o OUT] [--dark] [--highlight q1,q2]` | draw |
| `layout FILE --algorithm A -o OUT` | circle, gem, random, spiral, tree, tree-degree, two-circle |
| `grammar-type G` | regular, context-free, ..., CNF, GNF |
| `parse G INPUT... --parser brute\|brute-unrestricted\|ll1\|slr1\|cyk [--tree]` | derivation and tree |
| `tables G` | FIRST/FOLLOW, LL(1) table, SLR(1) states and ACTION/GOTO |
| `design (--constraint T \| --regex R \| --grammar G \| --template N) --alphabet ab [--accept w,..] [--reject w,..] [-o OUT] [--svg OUT]` | build a minimal automaton and verify it |
| `catedra definition FILE [--model mt\|all]` | the definition ⟨Q, Σ_E, Σ_A, q0, F, f⟩ of a one-tape MT or ALL, with warnings (a transition leaving a final state, nondeterminism) |
| `catedra table FILE [--markdown]` | the transition table of the course (Estado Actual \| Símbolo Entrada \| Estado Nuevo \| Símbolo Salida \| Movimiento Cabezal) |
| `catedra run FILE INPUT... [--all] [--model mt\|all] [--max-steps N] [--inputs-file F]` | configuration sequences `Δ1abaΔ ├─ ΔΔ2baΔ ... final/acepta` (exit 0 all accepted, 1 some rejected, 3 does not stop) |
| `catedra all-check FILE [INPUT...]` | the ALL rules, statically and with a run per input (exit 1 on violations) |
| `catedra compose A B [-o OUT]` | MT1 followed by MT2 |
| `membership G WORD [--max-forms N]` | membership by the sets Tk for non-contracting grammars (exit 0 member, 1 not) |
| `explain FILE` | plain-language summary |
| `lsystem FILE --depth N [-o OUT.svg]` | expand and draw an L-system |
| `pumping list`, `pumping play KIND LANG` | the pumping lemma games |
| `capabilities` | operations, document types, conversions, parsers, layouts |
| `gui [FILE] [--port P] [--no-open]` | serve the web editor on 127.0.0.1 and open it |

`rflap gui` embeds `web/dist/rflap.html` with `include_str!` and serves it on `127.0.0.1`
only. The document travels in the URL fragment (`/#doc=<base64url of the normalized JSON>`),
which the browser never sends to the server.

On Windows consoles that are not UTF-8, run `chcp 65001` (or use Windows Terminal) to see `λ`
and the tree characters correctly.

## Turing machines and ALL in the course's notation

`rflap catedra ...` follows the course "Sintaxis y Semántica de los Lenguajes" (Ing. Jorge
Buabud, UTN FRT): one-tape machines with `Δ` as the blank, moves `I` (left), `N` (stay) and
`D` (right), labels `(e,s,m)` and the configuration sequence `c1 ├─ c2 ...`. In a compact
document a label can be `(a,X,D)`, `a,X,D` or JFLAP's `a;X,R`; several labels of one edge are
written `(a,a,D) / (b,b,D)`; `"input_alphabet": "ab"` declares Σ_E (otherwise it is inferred).

```text
rflap catedra table palindrome.json
rflap catedra run palindrome.json aba ab λ
rflap catedra run igual.json ab --all          # linear bounded automaton: tape ⟨ab⟩
rflap catedra all-check igual.json ab b
rflap membership grammar.txt abbc              # T0, T1, ... until Tk == Tk-1
```

`catedra run`: model `mt` starts on a tape of `Δ` and shows the cells from one before the word to
one after it plus every cell the head visited (so λ gives `Δ1Δ ├─ ΔΔ8Δ`, one blank more than the
slide). Model `all` puts `⟨w⟩` on the tape and stops with a violation if the head leaves it or a
delimiter is overwritten. Entering a final state is `final/acepta`; no transition is
`bloqueo/rechazo` (`rechaza` for an ALL); too many steps is `no se detiene`.

## Building

The workspace targets the `windows-gnu` toolchain, where `windows-sys` cannot be built without
`dlltool`; `clap` is therefore used without its `color` feature (no ANSI colors in help).
