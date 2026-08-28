# El panel de control de batuta

**Estado: propuesta.** Nada de aquí se construye sin aprobación del Arquitecto.

Encargo: *«una interfaz donde se puedan seleccionar, habilitar, deshabilitar, eliminar y
agregar modelos, así como proveedores, y el esfuerzo de razonamiento por cada uno. Sencillo,
optimizado, fiable, liviano.»*

---

## §1 Lo primero es no confundir tres cosas

El panel enseña y edita tres capas distintas, y **la mitad de su valor está en no mezclarlas**:

| Capa | Qué es | Dónde vive | Quién la escribe |
|---|---|---|---|
| **Declaración** | qué ofrece un proveedor: sus modelos, sus nombres de ruta, su contención | `providers/*.toml` | una persona, a mano |
| **Evidencia** | qué funcionó de verdad, cuándo, y si el modelo quedó confirmado | `~/.local/state/batuta/recibos/` | `batuta canary`, nadie más |
| **Elección** | cuáles queremos usar, con qué esfuerzo, y cuáles no | `~/.local/state/batuta/politica.toml` | el panel |

Un modelo puede estar **declarado** y no tener **evidencia**: entonces no es enrutable, por
mucho que la **elección** diga que sí. R2 no se negocia desde el panel, y el panel tampoco
lo disimula: lo enseña en una columna.

**Habilitar y deshabilitar viven en la elección, nunca en el manifiesto.** Dos razones, y la
segunda es la que decide:

1. Que *nosotros* queramos usar un modelo no es una propiedad del proveedor. El manifiesto
   describe lo que hay; la política, lo que queremos.
2. **Los manifiestos llevan las mediciones en sus comentarios.** `USER` está en el `allow` de
   abacus con las ocho variables bisecadas escritas al lado; el nombre del modelo de visión
   lleva la lista que dio el propio proveedor. Un programa que reescriba TOML **borra los
   comentarios**, y en este repositorio los comentarios son el conocimiento. Un panel que
   editara manifiestos para habilitar un modelo destruiría lo que hace que el manifiesto
   valga.

---

## §2 Qué es el panel, concretamente

**Una orden que imprime una tabla, y unas cuantas que editan la elección.** Sin servidor,
sin TUI, sin dependencia nueva. «Liviano» aquí significa: se puede leer por `ssh`, se puede
`grep`, y no hay nada que arrancar.

```
$ batuta panel

PROVEEDOR  MODELO                            ESFUERZO  ESTADO   CANARIO           MODELO
dsh        deepseek-v4-flash                 alto      activo   verde  hace 2 h   confirmado
dsh        deepseek-v4-flash-vision-exp      medio     activo   verde  hace 2 h   confirmado
abacus     glm-5.3-flash                     —         activo   verde  hace 1 h   confirmado
abacus     glm-5.3                           —         apagado  verde  hace 1 h   confirmado
abacus     qwen3.8-max                       —         activo   verde  hace 1 h   confirmado  ⚠ QWEN3_8_MAX_THINKING
abacus     gemini-3.7-flash                  —         activo   verde  hace 1 h   confirmado  ⚠ GEMINI_3_7_FLASH_THINKING
abacus     routellm                          —         apagado  ninguno           —
```

La columna del canario es **evidencia con fecha**, no un adorno: si dice «ninguno», ese
modelo no se enruta aunque esté activo, y la tabla lo dice en la misma línea. El `⚠` marca
los modelos cuyo `observed_as` no coincide con lo pedido — Abacus resolvió a otra variante,
y quien mire el panel tiene que verlo sin ir a buscarlo.

Y una vista de un vistazo, para cuando la tabla no basta:

```
$ batuta panel --html /tmp/panel.html
```

Una página autocontenida, sin red, generada bajo demanda. **Sólo lectura**: lo que edita el
estado son las órdenes, que dejan rastro en el historial del shell y se pueden guionizar.

---

## §3 Las órdenes

```
batuta panel [--html <ruta>] [--provider <id>]

batuta enable  <proveedor>/<modelo>          activa un modelo
batuta disable <proveedor>/<modelo>          lo apaga, sin borrar nada
batuta effort  <proveedor>/<modelo> <nivel>  fija el esfuerzo de razonamiento

batuta nuevo-proveedor <id>                       plantilla comentada en providers/
batuta nuevo-modelo <proveedor> <id> <ruta>       añade un [[models]] al final
batuta quitar-modelo <proveedor>/<modelo>         lo borra e IMPRIME lo borrado
```

**Añadir un proveedor sigue siendo un fichero.** `nuevo-proveedor` no es un sistema de
configuración: es un andamio que escribe la plantilla con sus comentarios y sus huecos, para
que quien la rellene no tenga que recordar el esquema entero. La tesis no cambia.

**Añadir un modelo AÑADE al final**, nunca reescribe: así los comentarios de lo que ya
estaba se quedan donde están.

**Quitar imprime lo que quitó.** Es la única forma honesta de borrar de un fichero cuyos
comentarios llevan mediciones: si alguien borra el modelo que tenía anotada la bisección de
`USER`, al menos lo ve pasar por la pantalla y puede pegarlo de vuelta. Y `disable` existe
para que casi nunca haga falta borrar.

**Deshabilitar no borra el recibo.** La evidencia de que algo funcionó no deja de ser cierta
porque hayamos decidido no usarlo.

---

## §4 El esfuerzo de razonamiento

`ReasoningEffort` ya es un vocabulario cerrado de `batuta-contract` y **no lo usa nadie**.
Es literalmente el fallo que R13 describe: `allow_experimental` se validaba y nadie la pasaba
nunca, y era la única puerta de GLM 5.3.

Llega al proveedor por donde llega todo lo demás: una **sustitución**. `{reasoning_effort}`
se suma a las incorporadas, y cada manifiesto declara su mapa completo:

```toml
[substitutions.reasoning_effort]
low    = "low"
medium = "medium"
high   = "high"
```

Como todo mapa de sustitución, tiene que cubrir el vocabulario **entero**: si mañana se añade
un nivel, el manifiesto falla al cargar en vez de elegir en silencio.

**Lo que hay que medir antes de escribirlo**, porque no se sabe: dsh lo toma en
`agent-default-model.reasoningEffort` (se ve en el `settings.yaml` del anfitrión); de abacus
**no consta** que tenga bandera. Un proveedor sin esfuerzo declarable no declara el mapa, y
la columna sale `—`. Eso no es un hueco: es el dato.

---

## §5 Las tareas

Cada una de 20-30 minutos de cuerpo. El **contrato** —firmas y tests que fallan— no se
delega; el **cuerpo**, sí, por el flujo ya probado: worktree desechable, encargo ultra-literal
con criterio binario, y verificación de los comandos en esta máquina antes de integrar.

### T1 · `{reasoning_effort}`, medido primero — **hecho** (`b3b541d`)
- [x] **Medir** cómo toma dsh el esfuerzo, y si abacus lo toma de alguna forma *(medición, ~15 min)*.
      dsh: el adaptador de DeepSeek acepta exactamente `off`/`low`/`high`/`max`, nada más.
      abacus: `abacusai --help` (2.6.11) no ofrece ninguna bandera de esfuerzo.
- [x] `{reasoning_effort}` como llave incorporada, con `[substitutions.reasoning_effort]` cubriendo el vocabulario entero.
      Reservada: keyed por `ReasoningEffort`, no por `WriteMode` como el resto de sustituciones.
- [x] Test: un mapa incompleto **falla al cargar**
- [x] Test: un manifiesto sin el mapa carga, y pedir esfuerzo a ese proveedor es un error que lo dice
      (a nivel de carga: `{reasoning_effort}` sin mapa no es una llave admitida, R1)
- [x] `dsh.toml` lo declara (colapso hacia ABAJO: `medium`→`low`, `xhigh`→`high`, nunca hacia `max`);
      `abacus.toml` se queda sin mapa, con la medición que lo respalda
- [x] *(no estaba en la lista, lo pidió la revisión)* test que contrasta el mapa real de `dsh.toml` contra los
      cuatro literales medidos — cierra el hueco de precondición inerte: nada usa `{reasoning_effort}`
      todavía, así que la carga no comprueba los *valores* del mapa, sólo que esté completo

### T2 · `batuta-policy`: el fichero de elección — **hecho** (`2e4d60b`)
- [x] Crate nuevo. **No depende de `batuta-exec`** (R3), y hay un test que lee el `Cargo.toml`
      (`la_politica_no_depende_de_quien_mide`; sólo depende de `batuta-contract`)
- [x] `Politica::{cargar, guardar}` sobre una ruta explícita (`~/.local/state/batuta/politica.toml`
      es decisión de `Layout`, en `batuta-cli` — T2 no la fija, la recibe)
- [x] Por modelo: `habilitado: bool` y `esfuerzo: Option<ReasoningEffort>`
- [x] **Cero `Default`** (R13): un campo que nadie fija no compila
- [x] Un modelo que la política no menciona: decidir **una vez** si nace activo o apagado, y que el fichero lo diga por escrito.
      Decisión: **nace apagado** (misma disciplina que R5 en el entorno), documentada en el doc de módulo de `Politica`
- [x] Test: guardar y volver a cargar da lo mismo (ida y vuelta)
- [x] *(no estaba en la lista)* `schema_version` con `SchemaVersion::require_supported` (R1), y tests de
      entrada incompleta / versión no soportada / política recién estrenada sin modelos

### T3 · `batuta-store`: la evidencia, consultable — **hecho** (`dbf6ab3`)
- [x] `ReceiptStore::{open, latest_green}` sobre el directorio de recibos que ya existe
- [x] Invalidación por `manifest_sha256`: editar el manifiesto invalida sus recibos **sin que nadie tenga que acordarse**
      (el hash no coincide, y ese desacuerdo es la invalidación entera)
- [x] TTL **declarado y visible**, no constante mágica; el «caducado» dice cuándo caducó
      (`DEFAULT_TTL` público de 24h, documentado; `LatestGreen::Expired { at }` es el instante exacto)
- [x] Test: un recibo de otro `manifest_sha256` no cuenta
- [x] Test: R9 — leer no toma cerrojo, y el aserto es de tiempo (200 recibos, `Instant`/`elapsed` < 1s)
- [x] Un recibo ilegible **no** es un recibo ausente (`Lookup::unreadable`, separado del resultado)
- [x] *(no estaba en la lista)* `Receipt` gana `Deserialize` (antes sólo `Serialize`): es el único sitio
      donde deserializar un recibo está permitido, porque lee lo que `Receipt::seal` ya selló

### T4 · `batuta panel`: la tabla — **hecho** (`700cbf3`)
- [x] Une las tres capas: declaración, evidencia y elección
- [x] Columna de canario con **fecha relativa**, y `ninguno` cuando no lo hay
      (`hace N min/h/d`; `LatestGreen::Fresh` ganó `sealed_at` para poder calcularla)
- [x] Marca `⚠` cuando `observed_as` difiere de `route_model`
      (literal: cualquier alias declarado, no sólo los que resuelven a una variante distinta —
      acotarlo más pediría un normalizador nuevo, que T1 ya estableció que hay que evitar)
- [x] `--provider <id>` filtra (y uno inexistente enumera los que hay, R8, igual que `canary`)
- [x] Test: un modelo activo **sin recibo** sale enseñado y marcado como no enrutable
      (`ENRUTABLE` es su propia columna, separada de `ACTIVO`, tal como pedía §1; mutación
      verificada a mano para confirmar que el test detecta una regresión real)
- [x] Sin dependencia nueva: el ancho de columna se calcula a mano (`Anchos::de`)

### T5 · `enable`, `disable`, `effort` — **hecho** (`a540835`)
- [x] Cada una lee la política, la cambia y la guarda
- [x] `<proveedor>/<modelo>` inexistente ⇒ error que **enumera los que hay** (R8)
      (reutiliza `command::hallar`, el mismo helper que ya usaba `canary`)
- [x] `effort` a un proveedor sin mapa ⇒ error que lo dice, no un valor que se ignora
      (`Substitutions::declares_reasoning_effort` de T1, comprobado antes de escribir nada)
- [x] Test: `disable` no toca ni el manifiesto ni los recibos (byte a byte; mutación
      verificada a mano — la primera versión de la mutación apuntaba al manifiesto
      equivocado y no probaba nada, corregida antes de confiar en el resultado)
- [x] *(no estaba en la lista, lo encontró la prueba manual)* bug real: la primera
      elección de una instalación nueva fallaba porque `~/.local/state/batuta/` no
      existía todavía; arreglado igual que `canary` con `leases()`/`receipts()`

### T6 · `nuevo-proveedor`, `nuevo-modelo`, `quitar-modelo`
- [ ] `nuevo-proveedor` escribe una plantilla **comentada** que carga tal cual salvo los huecos evidentes
- [ ] `nuevo-modelo` **añade al final**; test: los comentarios previos sobreviven byte a byte
- [ ] `quitar-modelo` **imprime lo que borró**; test: lo impreso se puede volver a pegar y el manifiesto carga igual
- [ ] Ninguna de las tres reescribe el fichero entero

### T7 · `batuta panel --html`
- [ ] Página autocontenida: sin red, sin CDN, sin fuentes externas
- [ ] Los mismos datos que la tabla, y **la misma verdad**: un test compara las dos salidas
- [ ] Sólo lectura, y lo dice en la propia página

---

## §6 Lo que el panel NO va a hacer, y por qué

- **No lanza delegaciones.** Lanzar es `batuta canary` y será `batuta run`. Un panel que
  además ejecuta acaba informando de un estado que él mismo produce, y eso es R3.
- **No aplica parches.** `accept` y `reject` no están aquí por la misma razón por la que no
  están en el MCP: quien escribe un parche no se lo aprueba a sí mismo.
- **No edita manifiestos para habilitar.** Ver §1.
- **No guarda secretos.** R10: un secreto, un nombre, una vez. El panel enseña **nombres de
  credencial**, nunca valores, igual que el recibo enseña `env_names` y nunca los valores.
