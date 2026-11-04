# Testing

## Running every suite

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace                          # all Rust suites (unit, integration, doc tests)

cargo run -p xtask -- build-web                 # builds rflap-wasm and writes web/dist/rflap.html
node --test web/test/*.test.js                  # JavaScript suites (Node 22+; use the glob, not the directory)
```

`cargo test --workspace` needs no Java and no network. The CI (`.github/workflows/ci.yml`) runs exactly
this sequence. A single suite: `cargo test -p rflap-core --test convert`, a single test:
`cargo test -p rflap-api --test automata turing_tape`.

| Crate | Suites (`crates/<crate>/tests/`) |
|---|---|
| `rflap-core` | one file per module: `fa`, `fa_convert`, `pda`, `turing`, `mealy`, `grammar_*`, `convert`, `pda_form`, `regex`, `layout`, `render`, `lsystem*`, `pumping*`, `design_*`, `jff_*` |
| `rflap-api` | `automata`, `convert`, `grammar`, `misc`, `errors`, `snake_case` (no CamelCase enum value in a response) |
| `rflap-cli` | `run`, `convert_info`, `design_parse`, `misc` (exact stdout, stderr and exit codes) |
| `rflap-mcp` | `protocol` (every tool at message level, the widget fallback contract of `rflap_call`, the generated `docs/mcp-tools.md` kept in sync), `stdio`, `http` |
| `rflap-wasm` | `buffer_protocol` |

If the tool list or a schema changes, regenerate the reference and commit it:

```bash
cargo run -p rflap-mcp -- --tools-markdown > docs/mcp-tools.md
```

## Differential tests against JFLAP 7.1

Some expected outputs were **written by the real JFLAP 7.1 jar** running small Java harnesses, and
are committed under `crates/rflap-core/tests/fixtures/`. The Rust tests read them, so they run
without Java; they compare what Rflap computes with what JFLAP computed.

| Harness (Java) | Fixtures it writes | Rust test that reads them |
|---|---|---|
| `fixtures/t5/Harness.java` | `t5/expected.txt`: grammar type, λ/unit/useless removal, CNF, FIRST/FOLLOW, LL(1), SLR(1) | `grammar_jflap_diff.rs` |
| `fixtures/t15a/Harness.java` | `t15a/expected.txt`: FA↔grammar, PDA→CFG, CFG→PDA (LL, LR), TM→grammar | `convert_jflap_diff.rs` |
| `fixtures/t8/LHarness.java` | `t8/lsystem_expected.txt` and `t8/lsystem_*.jff` (expansions, image sizes, bounds, colours) | `lsystem_jflap_diff.rs` |
| `fixtures/t8/PHarness.java` | `t8/pumping_expected.txt` and `t8/pumping_*.jff` (the 26 languages, seeded choices) | `pumping_jflap_diff.rs` |
| `fixtures/jff/harness/HarnessAll.java`, `Harness.java` | `jff/jflap/*.jff` and `jff/sample*.jff`: files saved by JFLAP's own `XMLCodec` | `jff_fa.rs`, `jff_machines.rs`, `jff_structure.rs` |
| `fixtures/jff/harness/HarnessSim.java` | nothing is committed: it runs live, on files Rflap writes (see below) | `crates/rflap-api/tests/jflap_diff.rs` |

### Regenerating them

You need a local `JFLAP7_1.JAR` (the JFLAP 7.1 jar) and Java 17 or newer (the harnesses run as
single-file programs; Java 21 was used). The jar is **never committed** (`*.jar` is git-ignored); put
it anywhere and point `JAR` at it. Run from the directory of each harness, in bash (the
`$'\n'` makes JFLAP write LF line ends instead of CR LF on Windows):

```bash
JAR=/path/to/JFLAP7_1.JAR
cd crates/rflap-core/tests/fixtures

# grammars (T5) and cross-model conversions (T15a): print to stdout
(cd t5   && java -Djava.awt.headless=true -cp "$JAR" Harness.java > expected.txt)
(cd t15a && java -Djava.awt.headless=true -cp "$JAR" Harness.java > expected.txt)

# L-systems and pumping lemma (T8): the .jff files go to <out-dir>, the report to stdout
(cd t8 && java -Dline.separator=$'\n' -Djava.awt.headless=true -cp "$JAR" \
     LHarness.java . > lsystem_expected.txt)
(cd t8 && java -Dline.separator=$'\n' -Dfile.encoding=UTF-8 -Djava.awt.headless=true -cp "$JAR" \
     PHarness.java . > pumping_expected.txt)

# .jff of every type, written by JFLAP itself
(cd jff/harness && java -Djava.awt.headless=true -cp "$JAR" HarnessAll.java gen ../jflap)
```

Then `git diff` the fixtures and run `cargo test -p rflap-core`. **A change in a fixture means JFLAP
and Rflap disagree**: look at the diff before committing it. To check without overwriting, write to a
temporary directory and `diff` against the committed files (this is how the final cross-check of T14
was done: all of the above came out byte for byte the same as the committed files).

Both loaders also work the other way, to prove that JFLAP opens what Rflap writes:

```bash
RFLAP_JFF_EXPORT=/tmp/rflap-jff cargo test -p rflap-core --test jff_machines --test jff_structure exports_written
cd crates/rflap-core/tests/fixtures/jff/harness
for f in /tmp/rflap-jff/*.jff; do java -Djava.awt.headless=true -cp "$JAR" HarnessAll.java load "$f"; done

RFLAP_T8_OUT=/tmp/rflap-t8 cargo test -p rflap-core --test lsystem_jff --test pumping_jff -- --ignored
cd ../../t8
java -Dline.separator=$'\n' -Djava.awt.headless=true -cp "$JAR" LHarness.java load /tmp/rflap-t8/<file>.jff
java -Dline.separator=$'\n' -Djava.awt.headless=true -cp "$JAR" PHarness.java load /tmp/rflap-t8/<file>.jff
```

(`RFLAP_JFF_OUT=<dir> cargo test -p rflap-core --test jff_fa -- --ignored dump` does the same for
finite automata, with `Harness.java load`.) The harness prints what JFLAP read; a Java exception or a
different summary is a bug of the writer.

### Live tests against the jar (S8)

`crates/rflap-api/tests/jflap_diff.rs` runs `java` with `fixtures/jff/harness/HarnessSim.java` on files
that Rflap writes (designed automata, a PDA, Mealy, Moore, Turing machines, the ALL fixtures of the
course): JFLAP's `XMLCodec` loads them, the harness prints the graph and the verdict of JFLAP's own
simulators (FA with closure, PDA, Mealy, Moore and Turing step by state, acceptance by final state), and
the test compares both with Rflap's model and `simulate`. The jar is looked up in `$JFLAP_JAR`, then as
`JFLAP7_1.JAR` at the workspace root, then at the root of the main checkout of the repository (for linked
worktrees). **Without Java or the jar every test prints `SKIPPED` and passes**:

```bash
cargo test -p rflap-api --test jflap_diff -- --nocapture     # look for SKIPPED in the output
JFLAP_JAR=/path/to/JFLAP7_1.JAR cargo test -p rflap-api --test jflap_diff
```

(JFLAP's PDA simulator asks "Accept by" in a dialog from its constructor, so the harness builds it
without the constructor and sets final-state acceptance, which is what the dialog does.)

### Adding a case

Add it to the Java harness, regenerate the fixture as above, and add the matching assertion to the
Rust test. Write the expected value with the number of the JFLAP class that produced it; never copy
it from Rflap's own output.

## Conventions

* A test per rule: the name says the behaviour, the assertion is on the public interface (exit code,
  exact stdout and stderr, and that rejected input leaves the data unchanged).
* Rflap-only behaviour (design engine, `pda_jflap_form`, `tape_outputs`, the API envelope) is tested
  without fixtures: language equivalence against an independent predicate or brute force up to a fixed
  length (`pda_form.rs` checks the grammar against the PDA on every string up to length 6).
* Temporary files live in a fresh system temp directory and are removed by the test.
