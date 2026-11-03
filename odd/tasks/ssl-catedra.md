# Rflap: widget estilo JFLAP, español por defecto y MT/ALL según la bibliografía de la cátedra

Adapta las Máquinas de Turing y los ALL a la notación de la cátedra de Sintaxis y Semántica de los Lenguajes (Ing. Jorge Buabud, UTN FRT), y ajusta el widget: estilo JFLAP clásico, zoom automático, español por defecto y chequeo de ALL.
Fuentes: `C:\Users\lautaro\Downloads\CLASE 39 MT y ALL.pdf` y `CLASE 41 MT Especiales-LRE-LR.pdf`. Copias de texto e imágenes de las páginas en `reference/biblio/` (ignorado por git).

## Specs

- **S1** — "El de jflap clasico": el widget y el editor se dibujan siempre con el estilo clásico de JFLAP (fondo blanco, estados amarillos), aunque el chat esté en modo oscuro. La vista del editor y la imagen del asistente tienen el mismo estilo.
- **S2** — "ajustar zoom": al cargar o recibir un documento, el editor ajusta el zoom para que se vea el autómata completo, sin estados cortados en los bordes.
- **S3** — "poner español como idioma default": la interfaz arranca en español; el inglés queda como opción elegible.
- **S4** — "ok el chequeo": chequeo de ALL. Rflap verifica que una máquina cumple las reglas de un ALL de la cátedra:
  - Estáticas: f(q,⟨) = (q′,⟨,D) y f(q,⟩) = (q′,⟩,I) para toda transición que lea un delimitador; no se lee ni se escribe Δ.
  - Dinámicas: al simular, el cabezal nunca sale de ⟨…⟩.
  - Informa cada violación con la transición o el paso que la produce.
- **S5** — "necesito que se adapte a esta bibliografia por lo menos la parte de Maquina de Turing" (CLASE 39 MT y ALL, CLASE 41 MT Especiales-LRE-LR). Reglas tomadas de la bibliografía:
  - S5.1 Definición formal MT = ⟨Q, Σ_E, Σ_A, q0, F, f⟩ y ALL = ⟨Q, Σ_E, Σ_A, q0, F, f⟩; Σ_A incluye "Δ" (MT) o "⟨" y "⟩" (ALL). Rflap muestra los conjuntos de una máquina con esa forma. Σ_E se puede declarar; si no, se infiere.
  - S5.2 f : (Q-F) x (Σ_E∪Σ_A) ⇒ Q x (Σ_E∪Σ_A) x {I,N,D}: movimientos I (izquierda), N (nulo), D (derecha); blanco Δ. "El dominio no incluye a F, es decir que los estados finales son de parada."
  - S5.3 Aceptación y rechazo: llegar a un estado final es "final/acepta" (la máquina se detiene). Si no hay transición definida para el símbolo leído, hay "bloqueo/rechazo". Una MT puede no detenerse; un ALL siempre acepta o rechaza.
  - S5.4 Grafo de transición con etiquetas `(e,s,m)`; varias etiquetas en la misma arista se unen con " / " o se apilan, como en "(a , a , D) / (b , b , D)".
  - S5.5 Tabla de transición con columnas "Estado Actual | Símbolo Entrada | Estado Nuevo | Símbolo Salida | Movimiento Cabezal", "→" en el estado inicial, "*" en los finales y guiones en las filas de los finales.
  - S5.6 Configuración "c1...ci-1qkci...cn", operador "├─" y secuencia de configuración. Ejemplos de la bibliografía que Rflap reproduce exactos (MT del palíndromo, estados 1-8, F={8}):
    - "Δ1abaΔ ├─ ΔΔ2baΔ ├─ ΔΔb2aΔ ├─ ΔΔba2Δ ├─ ΔΔb3aΔ ├─ ΔΔ4bΔΔ ├─ Δ4ΔbΔΔ ├─ ΔΔ1bΔΔ ├─ ΔΔΔ5ΔΔ ├─ ΔΔ6ΔΔΔ ├─ ΔΔΔ8ΔΔ final/acepta"
    - "Δ1abΔ ├─ ΔΔ2bΔ ├─ ΔΔb2Δ ├─ ΔΔ3bΔ bloqueo/rechazo"
    - "Δ1bbΔ ├─ ΔΔ5bΔ ├─ ΔΔb5Δ ├─ ΔΔ6bΔ ├─ Δ7ΔΔΔ ├─ ΔΔ1ΔΔ ├─ ΔΔΔ8Δ final/acepta"
    - Regla de la ventana (deducida de los ejemplos): se muestran las celdas desde una antes de la palabra hasta una después (posiciones -1..n), más toda celda que el cabezal haya visitado. El estado va antes de la celda del cabezal.
    - Para w3 = λ la bibliografía escribe "Δ1Δ ├─ ΔΔ8". La regla da "Δ1Δ ├─ ΔΔ8Δ", y Rflap imprime eso. Es un desvío conocido y se documenta.
    - MT del palíndromo (CLASE 39, tabla y grafo), una fila por transición con estado, símbolo leído, estado nuevo, símbolo escrito y movimiento:
      - Estado 1: `1 a 2 Δ D`, `1 b 5 Δ D`, `1 Δ 8 Δ D`.
      - Estado 2: `2 a 2 a D`, `2 b 2 b D`, `2 Δ 3 Δ I`.
      - Estado 3: `3 a 4 Δ I`, `3 Δ 8 Δ D`.
      - Estado 4: `4 a 4 a I`, `4 b 4 b I`, `4 Δ 1 Δ D`.
      - Estado 5: `5 a 5 a D`, `5 b 5 b D`, `5 Δ 6 Δ I`.
      - Estado 6: `6 b 7 Δ I`, `6 Δ 8 Δ D`.
      - Estado 7: `7 a 7 a I`, `7 b 7 b I`, `7 Δ 1 Δ D`.
      - Inicial 1, F = {8}.
      - La fila `4 Δ 1 Δ D` falta en la tabla de la diapositiva, pero está en el grafo y la usa la secuencia de configuración.
  - S5.7 ALL: "Automáticamente se coloca en los extremos de esta secuencia los símbolos especiales “⟨” y “⟩” a la izquierda y derecha respectivamente". El cabezal empieza sobre el primer símbolo de la palabra. Ejemplos exactos del ALL "IGUAL" (estados I, A, B, C, D, F; Σ_A = {⟨, ⟩, X}):
    - "⟨Iab⟩ ├─ A⟨Xb⟩ ├─ ⟨BXb⟩ ├─ ⟨XBb⟩ ├─ ⟨CXX⟩ ├─ C⟨XX⟩ ├─ ⟨IXX⟩ ├─ ⟨XIX⟩ ├─ ⟨XXI⟩ ├─ ⟨XDX⟩ ├─ ⟨DXX⟩ ├─ D⟨XX⟩ ├─ ⟨FXX⟩ final/acepta"
    - "⟨Ib⟩ ├─ ⟨bI⟩ ├─ ⟨Db⟩ bloqueo/rechaza"
    - ALL "IGUAL" (CLASE 39, tabla y grafo), con el mismo formato de filas:
      - Estado I: `I a A X I`, `I b I b D`, `I X I X D`, `I ⟩ D ⟩ I`.
      - Estado A: `A b A b I`, `A X A X I`, `A ⟨ B ⟨ D`.
      - Estado B: `B a B a D`, `B b C X I`, `B X B X D`.
      - Estado C: `C a C a I`, `C X C X I`, `C ⟨ I ⟨ D`.
      - Estado D: `D X D X I`, `D ⟨ F ⟨ D`.
      - Inicial I, F = {F}.
    - En un ALL la configuración muestra la cinta completa entre ⟨ y ⟩, con el estado antes de la celda del cabezal; si el cabezal está sobre ⟨, el estado va antes de ⟨: "A⟨Xb⟩".
  - S5.8 Composición de MT (CLASE 41): MT3 = ⟨Q1∪Q2, Σ_E, Σ_A, q01, F2, f3⟩. f3 sigue a f1 en Q1, salvo que una transición de f1 que lleva a un estado de F1 pasa a llevar a q02, y sigue a f2 en Q2.
  - S5.9 Variantes (CLASE 41): MT multicinta (N cintas y cabezales independientes) y MT multipista ("los cabezales están restringidos a realizar todos juntos el mismo movimiento"). Rflap informa si una MT de varias cintas es multipista.
  - S5.10 Comprobación de pertenencia (CLASE 41) para gramáticas no compresoras: "T0 = { S }", "Tk = Tk-1 ∪ { β / |β| ≤ N ∧ α ∈ Tk-1 ∧ α ⇒ β }", "Repetir 2) hasta que: (Tk==Tk-1) o (w∈Tk)". Ejemplo de la bibliografía:
    - Gramática: N = { S, T, B, D }, T = { a, b, c }.
    - Producciones: 1) S → T, 2) DB → BD, 3) D → c, 4) T → aTBD, 5) T → abD, 6) bB → bb.
    - Con "abbc" (N = 4) el resultado es:
      - T0 = { S }
      - T1 = { S, T }
      - T2 = { S, T, aTBD, abD }
      - T3 = { S, T, aTBD, abD, aTBc, abc }
      - T4 = T3, así que "abbc" no pertenece.
  - S5.11 Claude, ChatGPT y los agentes explican y responden con esta notación (instrucciones del servidor MCP y skill).
- **S6** — "1. Si merge.": mergear el PR #2 (hecho), publicar la versión siguiente y actualizar `G:\rflap\bin`.

## Tasks

| ID | Specs | Qué | Ruta | Commit |
|----|-------|-----|------|--------|
| T1 | S4, S5.1-S5.10 | Backend: notación de la cátedra en core y api (etiquetas, tabla, configuraciones, ALL, chequeo, composición, multipista, pertenencia Tk), CLI | worker (worktree) | 0f941c3 |
| T2 | S1, S2, S3 | Web: estilo JFLAP clásico fijo, zoom automático, español por defecto | worker (worktree) | 30ea735 |
| T3 | S4, S5, S5.11 | Web con la notación y el ALL nuevos; instrucciones MCP y skill | worker (después de T1/T2) | bc68b98, be781e6 |
| T4 | S6 | Integrar, release v0.2.0, actualizar `G:\rflap\bin` | inline | |

## Log

- **L1** (pedido): "1. Si merge. 2. El de jflap clasico, ajustar zoom y poner español como idioma default, ok el chequeo. Tambien necesito que se adapte a esta bibliografia por lo menos la parte de Maquina de Turing: "C:\Users\lautaro\Downloads\CLASE 39 MT y ALL.pdf" "C:\Users\lautaro\Downloads\CLASE 41 MT Especiales-LRE-LR.pdf""
- **L2** (contexto): las preguntas eran "¿mergeo el PR #2 y publico la v0.1.1, así actualizo tu G:\rflap\bin?" y "¿hago las mejoras del widget? un solo estilo (JFLAP clásico o tema), zoom automático, chequeo de 'es un ALL válido'". PR #2 mergeado (`9f808a5`).
- **L3** (decisión): la notación de la cátedra es la que se usa por defecto. La notación JFLAP (`a;b,R`, □, L/S/R) queda como opción y se sigue aceptando al leer, porque la CLASE 41 muestra la MT multicinta en JFLAP. Una sola versión nueva (v0.2.0) junta el log y todo esto, así hay un solo reemplazo del `.exe`.
- **L4** (evidencia): T1 y T2 integrados (`f0f900a`): 680 tests de Rust y 145 de JS en verde. T1 reproduce exactas las secuencias de la bibliografía (palíndromo e IGUAL), la tabla, la definición formal y T0..T4 de "abbc"; agrega las ops `catedra_definition`, `catedra_table`, `catedra_run`, `all_check`, `compose_tm` y `membership_tk`. T2: estilo clásico fijo, `fitView` con botón Ajustar, español por defecto con selector de idioma.
- **L5** (evidencia): T3a (web) y T3b (MCP) integrados. Al integrar, arreglos propios con test previo:
  - `rflap_show` de una MT muestra solo la tabla de la cátedra.
  - El schema de `rflap_render` acepta `notation` y `join_labels`.
  - El resumen contaba 0 estados en las MT (bug: el grafo está anidado en `automaton`).
  - Q, F y la tabla siguen el orden de la bibliografía: inicial primero, luego orden natural.
  - Total: 711 tests de Rust y 166 de JS en verde. `rflap catedra run` reproduce las secuencias de la bibliografía.
  - Versión 0.2.0.
