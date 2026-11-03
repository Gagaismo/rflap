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
| T1 | S1, S5 | Workspace Cargo, LICENSE de JFLAP + atribución y contacto, README, .gitignore, CI; crear repo y pushear | inline | |
| T2 | S2.1 | Núcleo FA: modelo, etiquetas con rangos, simulación (paso a paso, clausura, rápida, múltiple), análisis (no determinismo, λ, inalcanzables, inútiles), estado trampa, combinar, equivalencia | inline | |
| T3 | S2.10 | E/S `.jff` para todos los tipos (ida y vuelta) | inline | |
| T4 | S2.2, S2.7 | ER (parser), NFA→DFA, minimizar, FA→ER, ER→FA, FA↔gramática regular | inline | |
| T5 | S2.6 | Gramáticas: modelo, tipo, transformaciones, FNC, parsers (fuerza bruta, LL(1), SLR(1), CYK, usuario), corridas múltiples | inline | |
| T6 | S2.3, S2.6 | PDA: modelo, simulación, PDA→CFG, CFG→PDA (LL y LR) | inline | |
| T7 | S2.4, S2.5 | MT de una y varias cintas, bloques, MT→gramática; Mealy y Moore | inline | |
| T8 | S2.8, S2.9 | L-systems y lema de bombeo | inline | |
| T9 | S2.10 | Layouts y render SVG | inline | |
| T10 | S4 | Motor de diseño: lenguaje (ER, gramática o especificación declarativa) → autómata mínimo verificado con ejemplos | inline | |
| T11 | S1, S2 | CLI `rflap` | inline | |
| T12 | S2, S3 | API WASM + editor web interactivo (editar, simular, convertir) | inline | |
| T13 | S3, S4 | Servidor MCP (stdio + HTTP) con herramientas y widget MCP Apps; guías para ChatGPT, Codex, Claude, Claude Code | inline | |
| T14 | S1-S5 | Verificación diferencial contra JFLAP (arnés Java), docs finales, push | inline | |

## Log

- **L1** (2026-11-02, pedido original): "Hola. Vamos a construir esto: Una version en Rust de Jflap. Jflap es open source, pero en particular me interesa que hagas en Rust la version del compilado que esta en la carpeta del proyecto. Vamos a agregar un par de funcionalidades. La idea es que sea compatible con Agentes y Chat de IAs (por ejemplo ChatGpt y codex y Claude y Claude code). Donde cuando se pida ayuda con algun ejercicio para la Materia de Sintaxis y Semantica de los Lenguajes, los automatas generados se puedan ver e interactuar directamente sobre el chat de la interfaz de ellos. Otra funcionalidad que quiero es que se pueda ingresar directamente un Lenguaje como entrada y que la IA funcione como el razonamiento conectado al Rflap y el se encargara del diseño. Quiero todas las mismas funcionalidades que ya existen por defecto en Jflap porteadas a Rust. Crea un proyecto en github y subi todo ahi."
- **L2** (decisiones del humano): sin orca-stack; "No tengo mucho espacio en disco local C, instalalo y configura para poder trabajar con Rust en mi disco G (SSD M2 se llama)"; repo "Público: Gagaismo/rflap"; "Hace un plan directo usando ODD".
- **L3** (evidencia): Rust 1.99.1 instalado en `G:\rust` (toolchain `stable-x86_64-pc-windows-gnu`, porque no hay MSVC Build Tools y C tiene 4,8 GB libres), con target `wasm32-unknown-unknown`; `target-dir = G:/rust/target`. El JAR trae solo bytecode, sin `.java`: descompilado con CFR a `reference/src` (465 archivos, 0 errores).
- **L4** (decisiones de diseño):
  - S3: Claude.ai y ChatGPT muestran un widget interactivo dentro del chat mediante un servidor MCP con recurso `ui://` (MCP Apps / Apps SDK).
  - Claude Code y Codex corren en la terminal y no muestran widgets: reciben un SVG, un diagrama en texto y un link a un visor local.
  - S4: la IA traduce el lenguaje natural a una especificación formal (ER, gramática o especificación declarativa con ejemplos) y Rflap construye, minimiza, dibuja y verifica.
  - La interfaz gráfica es web (WASM), y la misma sirve como widget del chat y como app de escritorio (`rflap gui`).
