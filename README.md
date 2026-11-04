# Rflap

Rflap is a Rust port of [JFLAP 7.1](https://www.jflap.org/), the tool for experimenting with formal
languages: finite automata, pushdown automata, Turing machines, Mealy and Moore machines, grammars,
regular expressions, L-systems and the pumping lemmas.

On top of the JFLAP features, Rflap is built to work with AI chats and agents (ChatGPT, Claude,
Claude Code, Codex):

- **See and interact with automata inside the chat.** The MCP server ships an interactive widget
  (MCP Apps) that ChatGPT and Claude render in the conversation. Terminal agents (Claude Code,
  Codex) get an SVG, a text diagram and a link to a local viewer.
- **Design from a language.** Give a language (a regular expression, a grammar or a declarative
  description with examples); the AI does the reasoning and Rflap builds, minimizes, lays out and
  **verifies** the automaton.

Everything is the same engine behind four front ends:

```text
                 rflap-core   (JFLAP algorithms, no I/O)
                      |
                 rflap-api    (one JSON request -> one JSON answer)
        +-------------+-------------+--------------+
   rflap-cli      rflap-mcp     rflap-wasm      (the web editor
   `rflap`        MCP server    web/dist/        runs rflap-wasm)
   terminal       stdio + HTTP  rflap.html
```

## Feature matrix: JFLAP 7.1 -> Rflap

Every environment and menu action of JFLAP 7.1 has an equivalent. The *CLI* column is the `rflap`
subcommand, the *MCP* column the tool of `rflap-mcp` (see [docs/mcp-tools.md](docs/mcp-tools.md)); the
operation behind both is the *API op* of `rflap capabilities`. The web editor (`rflap gui`) offers
the same actions in its menus. `rflap call` / `rflap_call` reach any API op directly.

### Finite automata

| JFLAP | CLI | MCP |
|---|---|---|
| Edit states, transitions, ranges `[0-9]` in labels | `rflap gui`, or write a `.jff` / compact JSON file | `rflap_show` |
| Step by state, step with closure | `rflap step F INPUT --steps N [--mode step\|closure]` | `rflap_step` |
| Fast run | `rflap run F INPUT` | `rflap_simulate` |
| Multiple run, batch | `rflap run F IN1 IN2 ...`, `--inputs-file` | `rflap_simulate` with `inputs` |
| Highlight nondeterminism, λ transitions, unreachable and useless states | `rflap info F`, `rflap render F --highlight q1,q2` | `rflap_analyze`, `rflap_render` |
| Add trap state to a DFA | `rflap convert F --to complete` | `rflap_convert` `complete` |
| Remove useless states | `rflap convert F --to remove-useless-states` | `rflap_convert` `remove_useless_states` |
| Combine automata | `rflap combine A B -o OUT` | `rflap_combine` |
| Compare equivalence | `rflap equiv A B` (prints a shortest counterexample) | `rflap_equivalent` |
| NFA to DFA | `rflap convert F --to dfa` | `rflap_convert` `dfa` |
| Minimize DFA | `rflap convert F --to minimal-dfa` | `rflap_convert` `minimal_dfa` |
| FA to regular expression | `rflap convert F --to regex` | `rflap_convert` `regex` |
| Regular expression to FA | `rflap convert --regex 'EXPR' --to nfa` | `rflap_convert` `nfa` |
| FA to regular grammar | `rflap convert F --to regular-grammar` | `rflap_convert` `regular_grammar` |
| Regular grammar to FA | `rflap convert --grammar 'TEXT' --to fa` | `rflap_convert` `fa` |

### Pushdown automata, Turing machines, Mealy and Moore machines

| JFLAP | CLI | MCP |
|---|---|---|
| PDA: run by final state or empty stack, push/pop of several symbols | `rflap run F INPUT --acceptance final\|empty-stack` | `rflap_simulate` `acceptance` |
| PDA to CFG | `rflap convert F --to cfg` | `rflap_convert` `cfg` |
| (Rflap) rewrite a PDA into the form PDA to CFG needs | `rflap convert F --to pda-jflap-form` (`cfg` does it by itself) | `rflap_convert` `pda_jflap_form` |
| Turing machine, one or several tapes, accept by final state or halting, building blocks | `rflap run F INPUT --acceptance final\|halting\|both`, `--tape T` once per tape | `rflap_simulate` (`tapes`, `acceptance`) |
| Turing machine to unrestricted grammar | `rflap convert F --to unrestricted-grammar` | `rflap_convert` `unrestricted_grammar` |
| Mealy and Moore machines with output | `rflap run F INPUT` (prints the output) | `rflap_simulate` |

### Grammars

| JFLAP | CLI | MCP |
|---|---|---|
| Test for grammar type | `rflap grammar-type G` | `rflap_grammar` (`type`) |
| Remove λ, unit and useless productions | `rflap convert G --to remove-lambda\|remove-unit\|remove-useless` | `rflap_grammar` / `rflap_convert` |
| Chomsky normal form | `rflap convert G --to cnf` | same, `cnf` |
| Brute force parse (restricted, unrestricted) | `rflap parse G INPUT --parser brute\|brute-unrestricted [--tree]` | `rflap_parse` |
| LL(1), SLR(1), CYK | `rflap parse G INPUT --parser ll1\|slr1\|cyk`, `rflap tables G` (FIRST/FOLLOW and both tables) | `rflap_parse`, `rflap_parse_tables` |
| User-controlled parse | `rflap call` with `user_parse` | `rflap_call` with `user_parse` |
| CFG to PDA (LL and LR) | `rflap convert G --to pda-ll\|pda-lr` | `rflap_convert` |
| Multiple parse (batch) | `rflap parse G IN1 IN2 ...` | `rflap_parse` with `inputs` |

### Everything else

| JFLAP | CLI | MCP |
|---|---|---|
| Regular expression editor | `--regex EXPR` / `regex:EXPR` on any command | `{"type":"regex",...}` documents |
| L-systems with 2D rendering | `rflap lsystem F --depth N -o OUT.svg` | `rflap_lsystem_render` |
| Pumping lemma games (26 predefined languages, both modes) | `rflap pumping list`, `rflap pumping play KIND LANG` | `rflap_pumping` |
| Layouts: circle, GEM, random, spiral, tree, two circles | `rflap layout F --algorithm A -o OUT` | `rflap_call` with `layout` |
| Open and save `.jff` (every type) | any command reads `.jff`; `-o OUT.jff` writes | `rflap_open_jff`, `rflap_save_jff` |
| Export image | `rflap render F -o OUT.svg` (also `--format md\|dot\|text`) | `rflap_render` |
| (Rflap) design an automaton from a language | `rflap design ...` | `rflap_design_automaton` |
| (Rflap) plain-language explanation | `rflap explain F` | `rflap_show` |

The deviations from JFLAP are listed in [docs/differences-from-jflap.md](docs/differences-from-jflap.md).

## Download

Prebuilt `rflap` and `rflap-mcp` for Windows, Linux and macOS are on the
[releases page](https://github.com/Gagaismo/rflap/releases/latest). Unzip the archive for your
system; it also has the integration guides and the agent skill. The binaries are not signed
(Windows: "More info → Run anyway"; macOS: `xattr -d com.apple.quarantine rflap rflap-mcp`).

## Install from source

You need a stable Rust toolchain (edition 2024, Rust 1.85 or newer) and git.

```bash
git clone https://github.com/Gagaismo/rflap
cd rflap
cargo build --release -p rflap-cli -p rflap-mcp
# target/release/rflap  and  target/release/rflap-mcp
cargo install --path crates/rflap-cli      # optional: puts `rflap` on your PATH
```

On Windows the project is developed with the **GNU toolchain** (`stable-x86_64-pc-windows-gnu`),
which needs no Visual Studio Build Tools:

```powershell
rustup toolchain install stable-x86_64-pc-windows-gnu
rustup default stable-x86_64-pc-windows-gnu
```

The MSVC toolchain was not tried on Windows (the CI runs on Linux). On a Windows console that is not UTF-8 run
`chcp 65001` (or use Windows Terminal) to see `λ` and the parse trees correctly.

`web/dist/rflap.html` (the editor, with the engine compiled to WebAssembly) is committed, so nothing
else is needed to run Rflap. To rebuild it you need the `wasm32-unknown-unknown` target
(`rustup target add wasm32-unknown-unknown`) and `cargo run -p xtask -- build-web`.

## Quick start: the command line

```bash
rflap capabilities                                   # everything it can do, with an example each
rflap design --constraint 'ends_with("ab")' --alphabet ab -o ends_ab.jff
rflap run ends_ab.jff aab abb                        # "aab": accepted / "abb": rejected, exit code 1
rflap render ends_ab.jff -o ends_ab.svg              # or --format md for a transition table
rflap convert --regex '(a+b)*abb' --to nfa -o nfa.jff
rflap convert nfa.jff --to minimal-dfa --steps       # the construction, step by step
rflap equiv nfa.jff 'regex:(a+b)*abb'                # equivalent (exit 0) or a counterexample (exit 1)
rflap tables --grammar 'E -> TA\nA -> +TA | λ\nT -> FB\nB -> *FB | λ\nF -> (E) | i'
```

Exit codes: `0` success (every input accepted), `1` some input rejected or not equivalent, `2` usage
or input error (`error: <code>: <message>` on stderr), `3` undecided (a budget ran out). `--json`
prints the raw answer of the API. A file is a `.jff` (any JFLAP type), a `.json` document, a grammar
in plain text (one production per line) or `-` for standard input. More in
[crates/rflap-cli/README.md](crates/rflap-cli/README.md) and `rflap <command> --help`.

## The web editor

```bash
rflap gui                  # serves the editor on 127.0.0.1 and opens the browser
rflap gui machine.jff      # opens a file
```

One self-contained file, `web/dist/rflap.html`, is the editor in a browser tab, the view inside MCP
Apps hosts (Claude) and the widget of ChatGPT. It runs the same engine as WebAssembly; when a host
blocks WebAssembly it calls the MCP tool `rflap_call` instead. The document travels in the URL
fragment, so it never leaves your machine. See [web/README.md](web/README.md).

## The MCP server

`rflap-mcp` speaks MCP over stdio (and Streamable HTTP with `--http`). Tools, the widget resource and
three prompts (`ssl_exercise`, `mt_catedra`, `grammar_tables`) are described in [docs/mcp-tools.md](docs/mcp-tools.md).

| Client | Guide |
|---|---|
| Claude Code | [docs/integrations/claude-code.md](docs/integrations/claude-code.md) |
| Codex | [docs/integrations/codex.md](docs/integrations/codex.md) |
| ChatGPT | [docs/integrations/chatgpt.md](docs/integrations/chatgpt.md) |
| Claude Desktop and Claude.ai | [docs/integrations/claude-desktop-and-claude-ai.md](docs/integrations/claude-desktop-and-claude-ai.md) |

Claude Desktop asks permission the first time each tool is used. If a call seems to hang, look for
the pending permission prompt in the chat, and choose *Always allow* for the Rflap tools.

For agents, [skills/rflap/SKILL.md](skills/rflap/SKILL.md) is a ready-made skill for solving
"Sintaxis y Semántica de los Lenguajes" exercises with the tools.

```bash
claude mcp add rflap -- /path/to/rflap-mcp --viewer-port 8787      # Claude Code
codex mcp add rflap -- /path/to/rflap-mcp --viewer-port 8787       # Codex
```

## Notación de la cátedra

Las Máquinas de Turing y los ALL se escriben, se explican y se simulan con la notación de la
bibliografía de la cátedra (definición `MT = ⟨Q, Σ_E, Σ_A, q0, F, f⟩`, etiquetas `(e,s,m)` con
movimientos `I`/`N`/`D` y blanco `Δ`, tabla de transición, secuencias de configuraciones con `├─`
terminadas en `final/acepta` o `bloqueo/rechazo`, chequeo de ALL con `⟨` `⟩`, composición de MT y
pertenencia por los conjuntos Tk):

```bash
rflap catedra definition palindromo.json
rflap catedra run palindromo.json aba ab        # Δ1abaΔ ├─ ΔΔ2baΔ ├─ … final/acepta
rflap catedra all-check igual.json ab b
rflap membership --grammar 'S -> T
DB -> BD
D -> c
T -> aTBD
T -> abD
bB -> bb' abbc
```

En el chat, las herramientas `rflap_mt_catedra`, `rflap_compose_tm` y `rflap_membership` y el prompt
`mt_catedra` hacen lo mismo. Guía completa: [docs/catedra.md](docs/catedra.md).

## Compatibility with classic JFLAP 7.1

Every automaton an AI designs, shows or saves through the MCP server must open in classic JFLAP 7.1
with the same states, initial state, finals, labels and symbols, and run with the same verdicts. The
API operation `jflap_check` reports what JFLAP cannot represent or runs differently (`issues`) and what
the user must know to use the file there (`notes`); the MCP tools append `⚠️ JFLAP 7.1: …` and
`ℹ️ JFLAP 7.1: …` lines to their replies and put the result in `structuredContent.jflap`. An ALL opens
the same but carries a note on how to run it in JFLAP (which has no ALL model). Details and the list of
findings: [docs/differences-from-jflap.md](docs/differences-from-jflap.md) and
[docs/catedra.md](docs/catedra.md).

## The design engine

You describe the language, Rflap builds the **minimal** automaton, names what each state remembers,
draws it and checks it against your examples and against a brute-force cross-check of every string
up to length 8. A failing example is reported, never hidden. With an AI, the AI translates the
exercise into the formal spec and Rflap does the rest. Three typical exercises:

**1. "Cadenas sobre {a,b} con cantidad par de a que terminan en b"** (regular, a constraint):

```bash
rflap design --constraint 'count(a) % 2 == 0 and ends_with("b")' --alphabet ab \
  --accept b,aab --reject λ,a,ab -o par_a_termina_b.jff
# Designed a DFA with 3 states ... q2 (accepting): count(a) is even, ends with "b"
# verification: ok (511 strings up to length 8 checked; examples 5/5 passed)
```

**2. "Dada la ER (a+b)*abb, obtener el AFD mínimo"** (regular expression):

```bash
rflap design --regex '(a+b)*abb' --alphabet ab --accept abb,aabb --reject ab,abba -o abb.jff
# a DFA with 4 states; `rflap convert --regex '(a+b)*abb' --to nfa` shows the construction of JFLAP
```

**3. "L = { aⁿbⁿ : n ≥ 1 } no es regular: dar un autómata de pila y su gramática"** (context-free):

```bash
rflap design --template an_bn --alphabet ab --param min=1 \
  --accept ab,aabb --reject λ,a,aab,abb -o anbn.jff        # a PDA, accepting by final state
rflap convert anbn.jff --to cfg                            # the grammar of that PDA
rflap pumping play regular 0 --mode computer_first         # why no finite automaton exists
```

`convert --to cfg` first rewrites the PDA into the form JFLAP's construction needs (one final state
reached on an empty stack, every transition pops one symbol and pushes none or two) and says so in
its steps. Other templates: `an_bm` (`n <= m`, `n >= m`, `n != m`), `equal_count`,
`balanced_brackets`, `wcwr`, `even_palindrome`, `palindrome`.

## Project layout

| Path | What |
|---|---|
| `crates/rflap-core` | JFLAP's algorithms: automata, simulation, conversions, grammars and parsers, `.jff`, layouts, SVG, L-systems, pumping lemma, design engine. No I/O. |
| `crates/rflap-api` | `Document` JSON of every type and the operations on it: one request in, one answer out. The only layer the front ends use. Compiles to WebAssembly. |
| `crates/rflap-cli` | the `rflap` command |
| `crates/rflap-mcp` | the MCP server (JSON-RPC by hand over stdio and HTTP) and the widget resource |
| `crates/rflap-wasm` | the API as a WebAssembly module (C ABI, no wasm-bindgen) |
| `crates/xtask` | `cargo run -p xtask -- build-web` builds `web/dist/rflap.html` |
| `web/` | the editor (vanilla ES modules, no dependencies) and its tests |
| `docs/`, `skills/` | integration guides, tool reference, the agent skill, JFLAP differences and testing |
| `odd/` | the plan and the evidence log of the port |

## Testing

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace                 # Rust suites
cargo run -p xtask -- build-web && node --test web/test/*.test.js     # JavaScript suites
```

The Rust suites include differential tests against **outputs written by the real JFLAP 7.1 jar**
(grammars, conversions, L-systems, the pumping lemma, `.jff` files). The expected outputs are
committed, so the tests need no Java; regenerating them does. See [docs/testing.md](docs/testing.md).

## License and attribution

Rflap is a modified version of JFLAP and is distributed free of charge under the JFLAP 7.0 license;
see [LICENSE](LICENSE). Rflap is not endorsed by the JFLAP authors or by Duke University. JFLAP was
written by Susan H. Rodger and her students at Duke University; `JFLAP7_1.JAR` is only used as a
reference, it is never committed (`*.jar` is git-ignored).

## En español

Rflap es un port a Rust de JFLAP 7.1 pensado para la materia **Sintaxis y Semántica de los Lenguajes**.
Hace lo mismo que JFLAP (autómatas finitos, de pila y de Turing, máquinas de Mealy y Moore, gramáticas,
expresiones regulares, L-systems y lema de bombeo), pero además:

- **Se usa desde el chat.** Si le pedís ayuda a ChatGPT, Claude, Claude Code o Codex con un ejercicio,
  la IA razona el lenguaje y le pide a Rflap que arme el autómata; en ChatGPT y Claude lo ves y lo
  manejás dentro del chat, y en la terminal recibís la tabla de transiciones, un dibujo SVG y un
  enlace para abrirlo en el navegador.
- **Diseña a partir de un lenguaje.** Escribís el lenguaje (por ejemplo "cadenas con cantidad par de
  a que terminan en b", una ER o una gramática) y Rflap devuelve el autómata **mínimo**, explica qué
  recuerda cada estado y lo **verifica** con ejemplos y con todas las cadenas de hasta 8 símbolos.

Cómo empezar como estudiante:

1. Instalá Rust (<https://rustup.rs>; en Windows usá la toolchain GNU, ver arriba) y compilá con
   `cargo install --path crates/rflap-cli`.
2. Probá en la terminal: `rflap design --constraint 'ends_with("ab")' --alphabet ab -o ab.jff`, luego
   `rflap run ab.jff aab abb` (acepta la primera, rechaza la segunda) y `rflap render ab.jff -o ab.svg`.
3. Para editarlo con el mouse: `rflap gui ab.jff`. Los archivos `.jff` también se abren en JFLAP.
4. Para usarlo desde un chat, seguí la guía de tu herramienta en `docs/integrations/` y pedile cosas
   como *"diseñá un AFD para las cadenas sobre {a,b} que terminan en ab y mostrame la tabla"* o
   *"hacé las tablas LL(1) de esta gramática"*. Pasos de una conversión (AFN→AFD, minimización,
   AFD→ER, gramática→AP) salen con `--steps` o en el panel de pasos del editor.

Para el parcial: `rflap convert` hace AFN→AFD, minimización, AF↔ER, AF↔gramática regular, AP↔GLC y
MT→gramática; `rflap parse` y `rflap tables` hacen fuerza bruta, LL(1), SLR(1) y CYK con los
conjuntos FIRST/FOLLOW; `rflap grammar-type` clasifica la gramática en la jerarquía de Chomsky;
`rflap pumping` juega el lema de bombeo.

## Known limitations

- The graphical editor covers automata of every type. Documents without a graph (grammars, regular
  expressions, L-systems, pumping games) open in a read-and-convert panel, and the dedicated panels
  for parsing, step-by-step replay, design and L-systems are still being finished; the CLI and the
  MCP server cover every operation today (`web/README.md` lists what the editor has).
- The MCP widget needs a host with MCP Apps support. Rendering in each host (Claude, ChatGPT,
  mobile apps) was designed from the specifications and tested with the protocol tests, but not
  every host was tried by hand; see the **UNVERIFIED** notes of the integration guides.
- The MCP specification of 2026-07-28 (without `initialize` and sessions) is not implemented; the
  server negotiates 2025-11-25 and earlier, which every client still offers.
- `rflap convert --to cfg` and `pda_jflap_form` assume a PDA that accepts by **final state**.
- Turing machine to grammar follows JFLAP's one-tape construction without building blocks.
- JFLAP's Swing-only features that are not computations (windows, dialogs, printing) are not
  reproduced; drawing is SVG.
- User-controlled grammar parsing has no dedicated CLI subcommand (use `rflap call` with `user_parse`).
