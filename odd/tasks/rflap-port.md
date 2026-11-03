# Rflap: port de JFLAP 7.1 a Rust, usable desde chats y agentes de IA

Feature: portar a Rust las funcionalidades de `JFLAP7_1.JAR`, exponerlas a ChatGPT, Codex, Claude y Claude Code (ver e interactuar con autómatas en el chat) y diseñar autómatas a partir de un lenguaje.
Referencia: `reference/src/` (JFLAP descompilado con CFR 0.152, ignorado por git). Repo: `github.com/Gagaismo/rflap` (público).

## Specs

- **S1** — "Una version en Rust de Jflap. Jflap es open source, pero en particular me interesa que hagas en Rust la version del compilado que esta en la carpeta del proyecto." La referencia es `JFLAP7_1.JAR` (JFLAP 7.1).
- **S2** — "Quiero todas las mismas funcionalidades que ya existen por defecto en Jflap porteadas a Rust." Inventario de JFLAP 7.1 (de `reference/src`):
  - S2.1 Autómatas finitos: edición, simulación paso a paso (con y sin clausura λ), corrida rápida, corridas múltiples y batch, rangos `[0-9]` en etiquetas, resaltado de no determinismo y de transiciones λ, estados inalcanzables/inútiles, agregar estado trampa a un DFA, combinar autómatas, comparar equivalencia (DFA equality).
  - S2.2 Conversiones de FA: NFA→DFA, minimizar DFA, FA→ER, ER→FA, FA→gramática regular, gramática regular→FA.
  - S2.3 Autómatas de pila (aceptación por estado final o pila vacía, push/pop de varios símbolos), PDA→CFG.
  - S2.4 Máquinas de Turing de una y varias cintas, aceptación por estado final o por parada, bloques de construcción, MT→gramática irrestricta.
  - S2.5 Máquinas de Mealy y de Moore con salida.
  - S2.6 Gramáticas: chequeo de tipo (regular, libre de contexto, irrestricta), eliminación de λ, unitarias e inútiles, FNC (Chomsky), parsers fuerza bruta (restringido e irrestricto), LL(1), SLR(1), CYK, parseo controlado por el usuario, CFG→PDA (LL y LR), corridas múltiples.
  - S2.7 Expresiones regulares (editor y conversión).
  - S2.8 L-systems con render 2D.
  - S2.9 Lema de bombeo regular y libre de contexto (los lenguajes predefinidos de JFLAP, modos usuario-primero y computadora-primero).
  - S2.10 Layouts de grafo (círculo, GEM, aleatorio, espiral, árbol, dos círculos), archivos `.jff` (leer y escribir todos los tipos), exportar imagen (SVG).
- **S3** — "La idea es que sea compatible con Agentes y Chat de IAs (por ejemplo ChatGpt y codex y Claude y Claude code). Donde cuando se pida ayuda con algun ejercicio para la Materia de Sintaxis y Semantica de los Lenguajes, los automatas generados se puedan ver e interactuar directamente sobre el chat de la interfaz de ellos."
- **S4** — "Otra funcionalidad que quiero es que se pueda ingresar directamente un Lenguaje como entrada y que la IA funcione como el razonamiento conectado al Rflap y el se encargara del diseño."
- **S5** — "Crea un proyecto en github y subi todo ahi." Repo público `Gagaismo/rflap` (L2).

## Tasks

| ID | Specs | Qué | Ruta | Commit |
|----|-------|-----|------|--------|
| T1 | S1, S5 | Workspace Cargo, LICENSE de JFLAP + atribución y contacto, README, .gitignore, CI; crear repo y pushear | inline | 3573968, 0b33d15 |
| T2 | S2.1 | Núcleo FA: modelo, etiquetas con rangos, simulación (paso a paso, clausura, rápida, múltiple), análisis (no determinismo, λ, inalcanzables, inútiles), estado trampa, combinar | inline | d47fa68 |
| T3 | S2.10 | Infraestructura `.jff` (leer/escribir) + autómatas finitos, ida y vuelta | inline | 8a9eae3 |
| T4 | S2.1, S2.2, S2.7 | ER (parser), NFA→DFA, minimizar, FA→ER, ER→FA, FA↔gramática regular, equivalencia de FA | inline | 2823a10 |
| T5 | S2.6 | Gramáticas: modelo, tipo, transformaciones, FNC, parsers (fuerza bruta, LL(1), SLR(1), CYK, usuario), corridas múltiples | inline | 4db0565 |
| T6 | S2.3 | PDA: modelo y simulación (estado final y pila vacía) | inline | 9149702 |
| T7 | S2.4, S2.5 | MT de una y varias cintas, filtros de aceptación, bloques; Mealy y Moore | inline | 84dede2 |
| T8 | S2.8, S2.9 | L-systems y lema de bombeo | inline | a0c5ec3 |
| T9 | S2.10 | Layouts y render SVG | inline | ce2277e |
| T10 | S4 | Motor de diseño: lenguaje (ER, gramática o especificación declarativa) → autómata mínimo verificado con ejemplos | inline | 463afd1 |
| T11 | S1, S2 | CLI `rflap` | inline | fb15bf8 |
| T12 | S2, S3 | T12a: crate `rflap-wasm`, editor web de autómatas (todos los tipos), simulación, archivos, puentes de host (navegador, MCP Apps, ChatGPT), build a un único `web/dist/rflap.html`; T12b: paneles de gramática, ER, conversiones paso a paso, parse, diseño, L-system y lema de bombeo | inline | 3009e0f, 8c48242, 3bab3d9 |
| T13 | S3, S4 | Servidor MCP (stdio + HTTP) con herramientas y widget MCP Apps; guías para ChatGPT, Codex, Claude, Claude Code | inline | 9050777 |
| T15 | S2.2, S2.3, S2.4, S2.6, S2.10 | Conversiones cruzadas: FA↔gramática regular, PDA→CFG, CFG→PDA (LL y LR), MT→gramática irrestricta; `.jff` de PDA, MT, Mealy, Moore, gramática, ER, L-system y lema de bombeo | inline | f40d2b1, 4218fe4 |
| T16 | S2, S3, S4 | Crate `rflap-api`: `Document` JSON de todos los tipos (con .jff), operaciones (simular, convertir, analizar, layout, render, diseñar) como capa común para CLI, WASM y MCP | inline | ab85dd6 |
| T14 | S1-S5 | Verificación diferencial contra JFLAP (arnés Java), docs finales, push | inline | 8476e5a |

## Log

- **L1** (2026-11-02, pedido original): "Hola. Vamos a construir esto: Una version en Rust de Jflap. Jflap es open source, pero en particular me interesa que hagas en Rust la version del compilado que esta en la carpeta del proyecto. Vamos a agregar un par de funcionalidades. La idea es que sea compatible con Agentes y Chat de IAs (por ejemplo ChatGpt y codex y Claude y Claude code). Donde cuando se pida ayuda con algun ejercicio para la Materia de Sintaxis y Semantica de los Lenguajes, los automatas generados se puedan ver e interactuar directamente sobre el chat de la interfaz de ellos. Otra funcionalidad que quiero es que se pueda ingresar directamente un Lenguaje como entrada y que la IA funcione como el razonamiento conectado al Rflap y el se encargara del diseño. Quiero todas las mismas funcionalidades que ya existen por defecto en Jflap porteadas a Rust. Crea un proyecto en github y subi todo ahi."
- **L2** (decisiones del humano): sin orca-stack; "No tengo mucho espacio en disco local C, instalalo y configura para poder trabajar con Rust en mi disco G (SSD M2 se llama)"; repo "Público: Gagaismo/rflap"; "Hace un plan directo usando ODD".
- **L3** (evidencia): Rust 1.99.1 instalado en `G:\rust` (toolchain `stable-x86_64-pc-windows-gnu`, porque no hay MSVC Build Tools y C tiene 4,8 GB libres), con target `wasm32-unknown-unknown`; `target-dir = G:/rust/target`. El JAR trae solo bytecode, sin `.java`: descompilado con CFR a `reference/src` (465 archivos, 0 errores).
- **L4** (decisiones de diseño):
  - S3: Claude.ai y ChatGPT muestran un widget interactivo dentro del chat mediante un servidor MCP con recurso `ui://` (MCP Apps / Apps SDK).
  - Claude Code y Codex corren en la terminal y no muestran widgets: reciben un SVG, un diagrama en texto y un link a un visor local.
  - S4: la IA traduce el lenguaje natural a una especificación formal (ER, gramática o especificación declarativa con ejemplos) y Rflap construye, minimiza, dibuja y verifica.
  - La interfaz gráfica es web (WASM), y la misma sirve como widget del chat y como app de escritorio (`rflap gui`).
- **L5** (plan): la equivalencia de FA pasa de T2 a T4 porque depende de NFA→DFA y de minimizar. Desvío de JFLAP: en el simulador con clausura, una transición con rango `[x-y]` toma la clausura λ del destino (JFLAP no lo hace); un rango solo se reconoce con la forma exacta `[x-y]`.
- **L6** (plan): olas. Ola 1 en paralelo con worktrees aislados en `.worktrees/` y un `CARGO_TARGET_DIR` por worker: T3, T4, T5, T6, T7 y T9. Ola 2: T8 y T15 (nueva: conversiones que cruzan modelos y el `.jff` de los demás tipos). Ola 3: T10 a T13; al final, T14. Cada worker es dueño de su archivo de módulo, ya declarado en `lib.rs`.
- **L7** (evidencia y plan): ola 1 integrada en `feat/rflap-port`: 224 tests en verde y clippy limpio. T5 coincide con el JAR real en λ, unitarias, inútiles, FNC, FIRST/FOLLOW, LL y SLR. Ola 2: T8, T10, T15a (conversiones cruzadas en `src/convert/`) y T15b (`.jff` de PDA, MT, Mealy, Moore, gramática y ER). Ola 3: T16 (`rflap-api`). Ola 4: T11, T12 y T13 en paralelo sobre `rflap-api`. Al final, T14.
- **L8** (evidencia): ola 2 integrada, 385 tests en verde. Contra el JAR real coinciden: L-systems (expansiones, colores, archivos byte a byte), los 26 lenguajes del lema de bombeo, las 5 conversiones cruzadas y la carga en JFLAP de los `.jff` de todos los tipos. Hallazgo: JFLAP 7.1 no recarga sus propios archivos con bloques de MT anidados que usan los tags por defecto (`StackOverflowError`); Rflap escribe tags únicos.
- **L9** (evidencia y decisiones): T16 integrado, 477 tests en verde y `rflap-api` compila a wasm32. Spec MCP Apps verificada en la fuente (github.com/modelcontextprotocol/ext-apps, `specification/2026-01-26/apps.mdx`, copia en `reference/mcp/`): `_meta.ui.resourceUri`, `text/html;profile=mcp-app`, `ui/initialize`, `ui/notifications/tool-result`, `ui/message`. Los ejemplos oficiales de OpenAI (openai-apps-sdk-examples) usan el mismo `_meta.ui.resourceUri`. Existe rmcp v3.8.0, pero se elige JSON-RPC a mano (tiny_http + stdio) porque son pocos métodos y evita tokio/axum en el toolchain GNU. Ola 4: T11, T12a y T13 en paralelo; después T12b. El widget es el mismo HTML autocontenido `web/dist/rflap.html` (commiteado) que `rflap gui` y `rflap-mcp` embeben con `include_str!`.
- **L10** (evidencia): ola 4 integrada (`b69a7cf`): 598 tests de Rust y 61 de JS en verde. Smoke de punta a punta con los binarios release:
  - `rflap-mcp` por stdio negocia la versión 2025-11-25.
  - `rflap_design_automaton` para `count(a) % 2 == 0 and ends_with("b")` devuelve un DFA mínimo de 3 estados con significados, ER, tabla y verificación de 511 cadenas.
  - `resources/read` entrega el editor real (2,7 MB, `text/html;profile=mcp-app`).
  - `rflap design --regex '(a+b)*abb'` da 4 estados y `run` sale con 1 ante un rechazo.
  - Seguimiento: se publicó la spec MCP 2026-07-28 (sin `initialize` ni sesiones). Los clientes vuelven a 2025-11-25, así que no bloquea.
  - T12a commiteado en `3009e0f`; T12b y T14 en paralelo.
- **L11** (evidencia de cierre): T12b (paneles de gramática, ER, diseño, L-system, lema de bombeo y replay) y T14 integrados. T14 cerró los huecos: `tape_outputs` de MT, contrato `rflap_call` del widget con `visibility` app, enums en snake_case, `combine` y `remove_useless_states` para MT, y `pda_to_jflap_form` con `convert cfg` automático. Escribió `README.md`, `docs/differences-from-jflap.md` y `docs/testing.md`. Al integrar, el JS esperaba valores CamelCase: arreglado en `3bab3d9` (snake_case canónico; acepta ambos). Verificación final:
  - 613 tests de Rust y 124 de JS en verde; clippy y fmt limpios.
  - El bundle `web/dist/rflap.html` es reproducible (2,79 MiB, mismo SHA-1 al reconstruir).
  - CI de GitHub `success` en `3bab3d9`.
  - Los harnesses contra el JAR real (T5, T15a, T8, HarnessAll) siguen idénticos a JFLAP.
  - Pendiente del humano: probar el widget en hosts reales (Claude Desktop/claude.ai, ChatGPT con túnel HTTPS) y decidir el merge a `master`.
