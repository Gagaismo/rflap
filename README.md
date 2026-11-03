# Rflap

Rflap is a Rust port of [JFLAP 7.1](https://www.jflap.org/), the tool for
experimenting with formal languages: finite automata, pushdown automata,
Turing machines, Mealy and Moore machines, grammars, regular expressions,
L-systems and the pumping lemmas.

On top of the JFLAP features, Rflap is built to work with AI chats and agents
(ChatGPT, Claude, Claude Code, Codex):

- **See and interact with automata inside the chat.** The MCP server ships an
  interactive widget (MCP Apps) that ChatGPT and Claude render in the
  conversation. Terminal agents (Claude Code, Codex) get an SVG, a text
  diagram and a link to a local viewer.
- **Design from a language.** Give a language (a regular expression, a grammar
  or a declarative description with examples); the AI does the reasoning and
  Rflap builds, minimizes, lays out and verifies the automaton.

> Status: under active development. See `odd/tasks/rflap-port.md` for the plan.

## En español

Rflap es un port a Rust de JFLAP 7.1 pensado para la materia Sintaxis y
Semántica de los Lenguajes: los autómatas que arma una IA (ChatGPT, Claude,
Claude Code, Codex) se ven y se pueden manejar dentro del chat, y se puede
pedir un autómata a partir de un lenguaje.

## License

Rflap is a modified version of JFLAP and is distributed free of charge under
the JFLAP 7.0 license; see [LICENSE](LICENSE). Rflap is not endorsed by the
JFLAP authors.
