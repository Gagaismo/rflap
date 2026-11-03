# Rflap: correcciones del handoff de feedback del 3 de nov (ejercicio 2.1, ALL)

Correcciones pedidas por Lautaro tras resolver el ejercicio 2.1 (cinco ALL) en Claude Desktop con el MCP de Rflap.
Rama `feat/handoff-feedback`. Decisiones del humano: implementar directo con ODD; `<` y `>` son alias de ⟨ ⟩ solo en modelo ALL.

## Specs

- **S1** — Punto 1, "all_check se colgó dos veces (4 min cada una, sin respuesta)". Diagnóstico (L2): la llamada nunca salió de Claude Desktop (`tool_approval_gate ... "approvalRequired":true`); la API responde en 60 ms y rflap-mcp por stdio en 10 ms con `ok: true`. Lo que queda:
  - S1.1 Test de regresión con la llamada de reproducción exacta del handoff (máquina del inciso a, `input_alphabet: "ab"`, `model: "all"`, las 12 entradas) por `rflap_mt_catedra`: "responde en menos de 5 s con ok: true y los veredictos esperados": rechaza "λ, b, ba, aba, ab, aabb"; acepta "a, aa, aab, abb, aaabb, abbb".
  - S1.2 "rflap_mt_catedra devuelve error explícito ante cualquier falla interna, nunca silencio" y "Respetar max_steps como límite duro".
  - S1.3 Documentar en docs/README que Claude Desktop pide permiso la primera vez que se usa cada herramienta y que conviene "Permitir siempre" las herramientas de Rflap.
- **S2** — Títulos: "Aceptar un campo title (o name) en el documento compacto y mostrarlo en la pestaña y el encabezado del widget." "Mostrar el título también en la primera línea del resultado de texto, para que el modelo lo vea." Hoy falla con "invalid_document: unknown field \"name\" in a compact turing document". `name` es alias de `title`.
- **S3** — "Decisión: mantener un widget por máquina, no un solo widget con varias pestañas. Cada widget debe distinguirse a simple vista: título visible en la cabecera y sobre el diagrama, en lugar de la pestaña genérica «Del asistente»." Checklist: "Cinco rflap_show seguidos se distinguen a simple vista." Sin título, la pestaña sigue diciendo «Del asistente».
- **S4** — Layout: "Por defecto, ubicar el estado inicial en el extremo izquierdo y ordenar el resto por distancia BFS desde él." Vale cuando el documento compacto no da `layout`; los algoritmos existentes siguen disponibles por nombre y `points` sigue ganando. "Documentar los valores aceptados de layout y el formato de points en el documento compacto; hoy no se pueden descubrir."
- **S5** — Pestaña «Asistente»: "Muestra el texto crudo del resultado: la tabla markdown aparece aplanada en una línea con | y Ø, seguida de la palabra tool rflap_show. Debería renderizarse como tabla o mostrarse resumida." No se muestra la clave `tool` (ni `title`).
- **S6** — ALL fuera de rflap_mt_catedra: "Agregar model: \"all\" (o detección por ⟨ ⟩) a simulate, step y show, con los delimitadores colocados automáticamente." "En modo ALL, avisar en el resultado cualquier transición que use Δ o que pueda salir de ⟨…⟩." Modelo de la cátedra: "cinta ⟨w⟩, cabezal sobre el primer símbolo y prohibido salir de los límites."
- **S7** — Notación: "Aceptar etiquetas de cátedra (e,s,m) como entrada en todas las herramientas, no solo en rflap_mt_catedra." (el documento compacto ya las acepta; falta probarlo y decirlo en las descripciones). "Hacer que la notación de salida sea configurable y coherente entre el texto devuelto y el widget."
- **S8** — "< y > funcionan como alias de ⟨ y ⟩." Decisión del humano (L3): solo en modelo ALL (`model: "all"` explícito, o `all_check`) y solo si la máquina no usa ⟨ ⟩; en una MT común `<` y `>` siguen siendo símbolos de la cinta.
- **S9** — Punto 8, "Resumen «Turing machine with 0 states and 0 transitions» erróneo": ya corregido en 0.2.0 (la tanda con 0 estados corrió con la 0.1.0). Falta el test: "El resumen del widget nunca informa 0 estados para una máquina no vacía."
- **S10** — "rflap_capabilities(section: \"layout\") devolvió una respuesta vacía sin error. Debería listar las secciones válidas (operations, document_types, convert)."
- **S11** — "El análisis de las máquinas turing siempre muestra alphabet: none. Debería inferir Σ_E y Σ_A como ya lo hace catedra_definition."
- **S12** — Descripciones: "En rflap_simulate, rflap_step y rflap_show: agregar una línea del tipo «para ALL o ejercicios en notación de la cátedra (UTN FRT), usá rflap_mt_catedra»." "En rflap_mt_catedra: incluir las palabras clave «ALL», «autómata linealmente acotado» y «linear bounded automaton» al principio de la descripción". "En las respuestas de show para turing: si no hay ⟨ ⟩, agregar una nota corta que indique cómo verificar la máquina como ALL." "Documentar en la descripción de show los campos opcionales del documento compacto (layout, points, y title cuando exista)."

## Tasks

- [ ] T1 — core+api: layout por defecto con inicial a la izquierda (S4), alias `<` `>` en ALL (S8), Σ_E/Σ_A en el análisis de turing (S11), `title`/`name` aceptados en el compacto (S2), etiquetas (e,s,m) probadas (S7). Ruta: worker en worktree `t1`. Commit: —
- [ ] T2 — mcp: título en texto y structured (S2), model all en simulate/step/show y avisos (S6), capabilities inválida (S10), descripciones (S12, S4 doc), regresiones S1.1/S1.2/S9, notación de salida (S7), docs S1.3. Ruta: worker en worktree `t2`. Commit: —
- [ ] T3 — web: título en pestaña y sobre el diagrama (S2, S3), pestaña Asistente legible (S5). Ruta: worker en worktree `t3`. Commit: —
- [ ] T4 — integrar t1-t3, rebuild del widget (`cargo run -p xtask -- build-web`), suite completa, verificación por S#. Ruta: inline. Commit: —

## Log

- **L1** — Pedido original (3 nov 2026): "Te mando un handoff con unas pruebas de ayer sobre Rflap que hice con algunos ejercicios de ALL en claude." Handoff completo: «Rflap — Handoff de feedback y correcciones», 11 puntos y checklist de aceptación (citados en Specs).
- **L2** — Diagnóstico del punto 1: `%LOCALAPPDATA%\Claude\Logs\claude.ai-web.log` muestra `tool_approval_gate {"toolName":"rflap:rflap_mt_catedra","approvalRequired":true}` a las 18:01 y 18:06; `mcp-server-rflap.log` no registra ninguna llamada a `rflap_mt_catedra`. `rflap call` responde en 60 ms y `rflap-mcp` por stdio en 10 ms, `ok: true`, 12/12 veredictos esperados.
- **L3** — Respuestas del humano: "Implementar directo con ODD (Recomendado)"; `<`/`>`: "Solo en modelo ALL (Recomendado)".
