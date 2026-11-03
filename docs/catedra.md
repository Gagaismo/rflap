# Notación de la cátedra: Máquinas de Turing y ALL

Guía para estudiantes de **Sintaxis y Semántica de los Lenguajes** (UTN FRT). Rflap escribe y
explica las Máquinas de Turing (MT) y los Autómatas Linealmente Acotados (ALL) con la notación de
la bibliografía de la cátedra (CLASE 39 *MT y ALL* y CLASE 41 *MT Especiales, LRE y LR*), no con la
de JFLAP. La notación de JFLAP se sigue leyendo: es la de las máquinas multicinta de la CLASE 41.

## Cátedra ↔ JFLAP

| Concepto | Cátedra | JFLAP |
|---|---|---|
| Blanco | `Δ` | `□` (en Rflap, `_` o `Δ`) |
| Movimientos | `I` izquierda, `N` nulo, `D` derecha | `L`, `S`, `R` |
| Etiqueta de una transición | `(e,s,m)`: lee `e`, escribe `s`, mueve `m` | `e;s,m` |
| Varias etiquetas en una arista | `(a,a,D) / (b,b,D)` | una etiqueta por línea |
| Varias cintas | no se usa | `a;a,R\|_;x,S` (una acción por cinta, separadas por `\|`) |
| Estados finales | de parada: `f` no está definida en ellos | igual |
| Rechazo | *bloqueo*: no hay transición para el símbolo leído | igual |

Rflap acepta las dos escrituras al leer; al mostrar usa la de la cátedra.

## Cómo se escribe una MT o un ALL

Un documento `turing` compacto (JSON). Las transiciones son `[desde, "etiqueta", hasta]`; `"states"` lista los
estados con los nombres del ejercicio (`1`, `2`, … o `I`, `A`, `B`, …) y fija su orden. Σ_E se puede declarar con
`"input_alphabet": "ab"`; si falta, Rflap lo infiere: son auxiliares (Σ_A) las mayúsculas, `Δ`,
`⟨`, `⟩` y los símbolos que la máquina escribe y no son letras ni dígitos.

MT del palíndromo de la CLASE 39 (estados 1 a 8, `F = {8}`):

```json
{"type": "turing", "states": ["1", "2", "3", "4", "5", "6", "7", "8"],
 "initial": "1", "finals": ["8"],
 "transitions": [
   ["1", "(a,Δ,D)", "2"], ["1", "(b,Δ,D)", "5"], ["1", "(Δ,Δ,D)", "8"],
   ["2", "(a,a,D) / (b,b,D)", "2"], ["2", "(Δ,Δ,I)", "3"],
   ["3", "(a,Δ,I)", "4"], ["3", "(Δ,Δ,D)", "8"],
   ["4", "(a,a,I) / (b,b,I)", "4"], ["4", "(Δ,Δ,D)", "1"],
   ["5", "(a,a,D) / (b,b,D)", "5"], ["5", "(Δ,Δ,I)", "6"],
   ["6", "(b,Δ,I)", "7"], ["6", "(Δ,Δ,D)", "8"],
   ["7", "(a,a,I) / (b,b,I)", "7"], ["7", "(Δ,Δ,D)", "1"]]}
```

Un ALL se escribe igual, pero la cinta es `⟨w⟩`: la bibliografía coloca `⟨` y `⟩`
automáticamente a los extremos de la palabra y el cabezal empieza sobre su primer símbolo. ALL
«IGUAL» (misma cantidad de `a` y de `b`; estados I, A, B, C, D, F; `Σ_A = {⟨, ⟩, X}`; `F = {F}`):

```json
{"type": "turing", "states": ["I", "A", "B", "C", "D", "F"],
 "initial": "I", "finals": ["F"],
 "transitions": [
   ["I", "a,X,I", "A"], ["I", "(b,b,D) / (X,X,D)", "I"], ["I", "(⟩,⟩,I)", "D"],
   ["A", "(b,b,I) / (X,X,I)", "A"], ["A", "(⟨,⟨,D)", "B"],
   ["B", "(a,a,D)", "B"], ["B", "(b,X,I)", "C"], ["B", "(X,X,D)", "B"],
   ["C", "(a,a,I) / (X,X,I)", "C"], ["C", "(⟨,⟨,D)", "I"],
   ["D", "(X,X,I)", "D"], ["D", "(⟨,⟨,D)", "F"]]}
```

## Qué produce Rflap

Con la CLI (`rflap catedra …`) o con la herramienta MCP `rflap_mt_catedra` (acciones `definition`,
`table`, `run` y `all_check`). El modelo (`mt` o `all`) se detecta: es `all` si alguna transición
nombra `⟨` o `⟩`; se puede forzar con `--all` (CLI) o `model` (MCP).

### Definición formal

`rflap catedra definition palindromo.json`

```text
MT = ⟨Q, Σ_E, Σ_A, q0, F, f⟩
Q = {1, 2, 3, 4, 5, 6, 7, 8}
Σ_E = {a, b} (inferido)
Σ_A = {Δ}
q0 = 1
F = {8}
```

Σ_A incluye `Δ` en una MT y `⟨`, `⟩` en un ALL. Rflap avisa si la máquina es no determinista, si
`f` está definida en un estado final (los finales son de parada: el dominio de `f` es
`(Q-F) x (Σ_E∪Σ_A)`), si falta el estado inicial o si el Σ_E declarado no es válido.

### Tabla de transición

`rflap catedra table palindromo.json --markdown`

| Estado Actual | Símbolo Entrada | Estado Nuevo | Símbolo Salida | Movimiento Cabezal |
|---|---|---|---|---|
| →1 | a | 2 | Δ | D |
| →1 | b | 5 | Δ | D |
| →1 | Δ | 8 | Δ | D |
| … | … | … | … | … |
| 7 | Δ | 1 | Δ | D |
| *8 | - | - | - | - |

`→` marca el estado inicial (en cada una de sus filas), `*` los finales, y las filas de los finales llevan guiones. Las filas siguen el orden de los estados: para fijarlo, listalos en `"states"` del documento (si no, van en el orden en que aparecen en las transiciones). La
diapositiva de la CLASE 39 omite la fila `4 Δ 1 Δ D`, que sí está en el grafo y que usa la
secuencia de configuraciones; Rflap la incluye.

### Secuencias de configuraciones

Una configuración es `c1…ci-1 qk ci…cn` (el estado va antes de la celda del cabezal) y `├─` es un
paso. `rflap catedra run palindromo.json aba ab bb`:

```text
Δ1abaΔ ├─ ΔΔ2baΔ ├─ ΔΔb2aΔ ├─ ΔΔba2Δ ├─ ΔΔb3aΔ ├─ ΔΔ4bΔΔ ├─ Δ4ΔbΔΔ ├─ ΔΔ1bΔΔ ├─ ΔΔΔ5ΔΔ ├─ ΔΔ6ΔΔΔ ├─ ΔΔΔ8ΔΔ final/acepta
Δ1abΔ ├─ ΔΔ2bΔ ├─ ΔΔb2Δ ├─ ΔΔ3bΔ bloqueo/rechazo
Δ1bbΔ ├─ ΔΔ5bΔ ├─ ΔΔb5Δ ├─ ΔΔ6bΔ ├─ Δ7ΔΔΔ ├─ ΔΔ1ΔΔ ├─ ΔΔΔ8Δ final/acepta
```

* `final/acepta`: llegó a un estado final y la máquina se detiene.
* `bloqueo/rechazo`: no hay transición para el símbolo leído (en un ALL la bibliografía escribe
  `bloqueo/rechaza`).
* `no se detiene (límite de N pasos)`: una MT puede no detenerse; Rflap corta a los `--max-steps`
  pasos (1000 por defecto). Un ALL siempre acepta o rechaza.
* Ventana: se muestran las celdas desde una antes de la palabra hasta una después, más toda celda
  que el cabezal visitó. **Desvío conocido:** para la palabra vacía la bibliografía escribe
  `Δ1Δ ├─ ΔΔ8` y Rflap imprime `Δ1Δ ├─ ΔΔ8Δ`, que es lo que da la regla de la ventana.

En un ALL la configuración muestra la cinta completa entre `⟨` y `⟩`; si el cabezal está sobre `⟨`,
el estado va antes de `⟨`. `rflap catedra run igual.json ab b --all`:

```text
⟨Iab⟩ ├─ A⟨Xb⟩ ├─ ⟨BXb⟩ ├─ ⟨XBb⟩ ├─ ⟨CXX⟩ ├─ C⟨XX⟩ ├─ ⟨IXX⟩ ├─ ⟨XIX⟩ ├─ ⟨XXI⟩ ├─ ⟨XDX⟩ ├─ ⟨DXX⟩ ├─ D⟨XX⟩ ├─ ⟨FXX⟩ final/acepta
⟨Ib⟩ ├─ ⟨bI⟩ ├─ ⟨Db⟩ bloqueo/rechaza
```

### Chequeo de un ALL

`rflap catedra all-check igual.json ab b` (o `all_check` en MCP) verifica que la máquina cumpla las
reglas de un ALL y nombra cada violación:

* **Estáticas**: toda transición que lee `⟨` escribe `⟨` y mueve `D` (`f(q,⟨) = (q′,⟨,D)`); toda
  transición que lee `⟩` escribe `⟩` y mueve `I` (`f(q,⟩) = (q′,⟩,I)`); ninguna lee ni escribe `Δ`.
* **Dinámicas** (con palabras de prueba): al simular, el cabezal nunca sale de `⟨…⟩` ni se pisa un
  delimitador.

Cada violación dice la transición o el paso que la produce, por ejemplo:
`La transición I --(⟩,X,I)--> D lee ⟩: debe escribir ⟩ y moverse a la izquierda (I).` Sale con
código 0 si cumple y 1 si hay violaciones.

## Composición de MT

`rflap catedra compose a.json b.json -o ab.json` (o `rflap_compose_tm`) arma
`MT3 = ⟨Q1∪Q2, Σ_E, Σ_A, q01, F2, f3⟩`: `f3` sigue a `f1` en `Q1`, salvo que una transición de `f1`
que lleva a un estado de `F1` pasa a llevar a `q02`, y sigue a `f2` en `Q2`. Los estados de la
segunda máquina llevan una comilla (`s0'`). Con MT1 = «leer una `a`» y MT2 = «leer una `b`», MT3
corre `Δs0abΔ ├─ Δas0'bΔ ├─ Δabs1'Δ final/acepta`.

## Multicinta y multipista

Una MT multicinta tiene N cintas con cabezales independientes y se escribe en notación JFLAP, una
acción por cinta: `a;a,R|_;x,S`. Una MT **multipista** es la que «tiene los cabezales restringidos
a realizar todos juntos el mismo movimiento». `rflap_analyze` (MCP) y la operación `analyze` de `rflap call`
informan si una máquina de varias cintas es multipista y, si no, en qué transiciones los cabezales
se mueven distinto. La definición, la tabla y las secuencias son de una cinta; con varias, Rflap lo avisa.

## Pertenencia por los conjuntos Tk

Para una gramática **no compresora** (ninguna producción con `|α| > |β|`, salvo `S → λ`),
`rflap membership` (o `rflap_membership`) usa el algoritmo de la CLASE 41:
`T0 = { S }`, `Tk = Tk-1 ∪ { β / |β| ≤ N ∧ α ∈ Tk-1 ∧ α ⇒ β }` con `N = |w|`, repetido hasta que
`Tk = Tk-1` o `w ∈ Tk`. Ejemplo de la bibliografía, con `N = {S,T,B,D}`, `T = {a,b,c}` y las
producciones `S → T`, `DB → BD`, `D → c`, `T → aTBD`, `T → abD`, `bB → bb`, para `abbc`
(`N = 4`):

```text
T0 = { S }
T1 = { S, T }
T2 = { S, T, aTBD, abD }
T3 = { S, T, aTBD, abD, aTBc, abc }
T4 = T3
Como T4 = T3, "abbc" no pertenece al lenguaje.
```

## Con IA

Con el servidor MCP (Claude, ChatGPT, Claude Code, Codex) basta pedir, por ejemplo: *«Armá una MT
que reconozca palíndromos sobre {a,b}; mostrame la definición, la tabla y la secuencia para aba y
ab»*. El prompt `mt_catedra` arranca ese flujo desde un enunciado. Las herramientas están en
[mcp-tools.md](mcp-tools.md) y la skill para agentes en
[../skills/rflap/SKILL.md](../skills/rflap/SKILL.md).
