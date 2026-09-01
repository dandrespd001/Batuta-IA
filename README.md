# batuta

Orquestador de delegación en Rust. **Añadir un proveedor es un fichero, nunca un parche.**

El núcleo no conoce ningún proveedor: los lee de manifiestos declarativos releídos en
cada invocación.

Nació de un fallo concreto y medido: en el orquestador anterior, el transporte de un
proveedor estaba **declarado en un registro y ausente del otro**, así que toda tarea
enrutada allí moría *después* de pagar la corrida. De ahí sale la regla que ordena todo lo
demás: **nada se declara, se demuestra.**

## Navegación vigente

- [`specs/README.md`](specs/README.md) — autoridad viva, IDs y recorrido desde capacidad a requisito.
- [`specs/anchors.json`](specs/anchors.json) — inventario navegable de capacidades y verificaciones.
- [`ROADMAP.md`](ROADMAP.md) — siguientes slices, dependencias y autorización externa.
- [`CONTRIBUTING.md`](CONTRIBUTING.md) — flujo humano, impacto, recuperación y gates.
- [`AGENTS.md`](AGENTS.md) — instrucciones breves para agentes de implementación.
- [`docs/DOCUMENT_CLASSIFICATION.md`](docs/DOCUMENT_CLASSIFICATION.md) — clase, autoridad y
  mutabilidad de cada documento.

La precedencia es constitución, spec viva propietaria, schema vivo y después guías o historia. Los
documentos históricos de `docs/` conservan contexto y evidencia, pero no compiten con `specs/system/`.

## Estado

| Fase | Qué | Estado |
|---|---|---|
| **0** | **Desbloqueo y medición del transporte** | **hecho** (`docs/medidas/`) |
| **1** | **Workspace, `batuta-contract`, gates** | **hecho** |
| **2** | **Manifiestos** | **hecho** |
| **3** | **Ejecución, recibos, leases y el CLI** | **hecho** (`docs/medidas/CANARIOS.md`) |
| **4** | **Núcleo de política, calidad y routing** | **núcleo y cierre operativo K4 hechos; CLI general de política diferida** |
| **5** | **CLI JSON, MCP stdio y TUI local** | **superficies K4 hechas; CRUD TUI ajeno al perfil diferido** |
| 6 | Convivencia y corte | pendiente |

La adopción spec-anchored `RM-001` sigue `in_progress`: el checkpoint actual completa 37 de 42
tareas sin cambiar el estado del roadmap.

| Historia | Resultado | Estado |
|---|---|---|
| US1 | Autoridad viva, anchors, roadmap y clasificación navegables | hecha |
| US2 | Deriva, modularidad y arquitectura bloqueadas por el gate | hecha |
| US3 | Siete rutas V1 y 19 registros preservados; evidencia V2 separada | hecha |
| US4 | Autoría v1.0.2, guías y checker offline repetibles | hecha |
| Cierre T038–T042 | recorridos, evidencia final, aceptación y convergencia | pendiente |

El primer beneficio real llega en la Fase 3, no al final. Y llegó: `batuta canary` corre
contra dos proveedores reales y deja recibo. Los plugins de la Fase 3 original se aplazan
—no hay todavía nada que un plugin resuelva mejor que un manifiesto—.

## `batuta canary`

```sh
batuta canary --provider dsh --model dsh-deepseek-v4-flash
batuta canary --provider dsh --model dsh-deepseek-v4-flash --capability tools
batuta canary --provider abacus
```

La corrida más pequeña que demuestra que un proveedor responde. Genera un token
irrepetible, pide que lo devuelva, y comprueba que **volvió ése** — nunca busca una
subcadena en un juicio propio (R3). Toma los dos leases —por modelo y por repositorio—,
materializa los ficheros de corrida **fuera** del worktree, lanza el proceso en su propio
grupo, lee la procedencia del registro del proveedor y sella el recibo.

Tres códigos de salida, no dos:

| | |
|---|---|
| `0` | el canario salió verde |
| `1` | salió **rojo**: hubo veredicto y es negativo. El motivo se imprime |
| `2` | **no llegó a haber veredicto**. El motivo se imprime |

La distinción entre el `1` y el `2` es la misma que el recibo hace entre «no lo pude leer»
y «este proveedor no lo ofrece»: un canario rojo *es* una respuesta, y no haber llegado a
preguntar no lo es.

El estado vive en `$XDG_STATE_HOME/batuta` (o `~/.local/state/batuta`), con `leases/`,
`recibos/` y `corridas/`. Es *state* y no *data* a propósito: un lease sincronizado a otra
máquina describiría un proceso que allí no existe.

## Qué hay hoy

```
Cargo.toml                       workspace
crates/batuta-contract/          tipos, errores y vocabularios cerrados. CERO E/S
crates/batuta-manifest/          carga y validación de manifiestos
crates/batuta-receipt/           el recibo, que DERIVA su veredicto de los hechos
crates/batuta-lease/             admisión por leases, con caducidad por evidencia
crates/batuta-exec/              sustitución, materialización, árbol de procesos, canario
crates/batuta-quality/           observaciones, cestas, proyección y staging de investigación
crates/batuta-routing/           selector puro, salud, relevo y migración de política
crates/batuta-cli/               el binario `batuta`
providers/dsh.toml               DeepSeek Harness
providers/abacus.toml            Abacus.AI — el proveedor que originó el proyecto
pruebas/discrepante/dsh.toml     manifiesto deliberadamente equivocado: la prueba de
                                 que el recibo no miente
docs/ESQUEMA_MANIFIESTO.md       el esquema y su justificación
docs/REGLAS_INGENIERIA_RUST.md  reglas generales de modularidad y mantenibilidad
docs/DEUDA_MODULAR_RUST.md       registro de extracciones modulares pendientes y cerradas
docs/FASE3_EJECUCION.md          los siete criterios de la Fase 3, cerrados
docs/ESQUEMA_CALIDAD_ROUTING.md  contrato de evidencia y selección
docs/IMPLEMENTACION_ROUTING_V2.md estado verificable y límites operativos
docs/medidas/DSH_HEADLESS.md     lo que se midió del transporte, con las corridas
docs/medidas/DELEGACION_MANIFEST.md  la primera delegación real, verificada
docs/medidas/CANARIOS.md         los canarios reales, con sus recibos sin editar
specs/system/                    autoridad viva del comportamiento público
specs/anchors.json               capacidades, requisitos y verificaciones
AGENTS.md                        reglas de trabajo para agentes
CONTRIBUTING.md                  flujo de contribución humano
scripts_ci/local_gates.sh        los gates permanentes
.github/workflows/ci.yml         los mismos gates en CI
```

`batuta-contract` no depende de ningún otro crate de batuta y todos dependen de él.

**Nadie puede escribir un recibo verde a mano.** `Receipt::seal` recibe los hechos de la
corrida y **deriva** el veredicto de ellos, en el orden de la corrida: primero lo que
impidió ejecutar, luego lo que salió mal al ejecutar, y sólo al final lo que se ve leyendo
el registro. Un recibo que dijera «modelo equivocado» cuando el proceso ni arrancó estaría
diagnosticando mal.

**Un lease caduca por evidencia, nunca por antigüedad.** Se reclama sólo si se puede
demostrar que su dueño murió, leyendo `(pid, start_time)` de `/proc`. El campo
`acquired_at` no se consulta jamás.

`batuta-manifest` valida en dos mitades a propósito: `parse()` es **pura** —vocabularios,
formas, llaves, cobertura de mapas— y catorce de sus dieciocho pruebas corren sin tocar el
disco; `verify_executable()` es la que mira la máquina, y es la que paga R1: un ejecutor
irresoluble falla **al cargar**, no después de pagar la corrida.

Sus pruebas fijan los *mensajes* de error, no sólo los tipos —el que rechaza un `parser`
inválido exige que el mensaje liste los cuatro valores válidos—, porque un error que no
enumera lo que valía es el fallo que R8 paga.

## Gates

```sh
./scripts_ci/local_gates.sh
```

Es la única entrada agregada local y de CI. Ejecuta, en este orden, los once gates reales:

1. formato Rust;
2. atributo `no_std` de `batuta-contract`;
3. integridad offline de Spec Kit v1.0.2/codex y sus 22 ficheros administrados;
4. specs, anchors, impacto y correlación Git cuando se proporciona `BATUTA_SPEC_BASE`;
5. evidencia TDD V1/V2;
6. modularidad;
7. arquitectura de crates;
8. pruebas Python de los gates;
9. sidecar DSH offline;
10. Clippy con todo el workspace, targets y features;
11. tests Rust con todo el workspace y features.

Todos funcionan sin red, credenciales ni proveedores. CI hace checkout con historial completo y
llama sólo a este script. La compilación usa prioridad baja y dos jobs por defecto, como fija
[`AGENTS.md`](AGENTS.md); `BATUTA_BUILD_JOBS` es un override consciente.

### Por qué el gate de `no_std`

El brief exige que el contrato no haga entrada/salida. Declararlo en un comentario
no sirve de nada: `#![no_std]` hace que `std::fs`, `std::net`, `std::process` y
`std::time` **no existan** dentro del crate, así que la propiedad la comprueba el
compilador en cada compilación. El gate 2 sólo vigila que nadie retire el atributo.

Es R2 aplicada al propio código: nada se declara, se demuestra.

## Decisiones de la Fase 1 que la Fase 2 hereda

- **Los vocabularios cerrados se generan, no se escriben.** La macro
  `closed_vocabulary!` emite a la vez el enum, la lista de tokens, `FromStr`,
  `Display` y el par serde. No hay forma de declarar un vocabulario sin obtener
  también el error que enumera sus valores válidos (R8).
- **`ModelId` y `RouteModel` son tipos distintos.** El primero es el nombre
  canónico de batuta (`abacus-glm-5.3-flash`); el segundo, el que entiende el
  proveedor (`ZAI GLM 5.3 Flash`). Confundirlos es la clase de error que R10 paga.
- **`TaskSpec` sólo se obtiene validando.** Se rellena un `TaskSpecDraft` y se
  convierte con `TaskSpec::try_from`. Deserializar pasa por el mismo sitio, así
  que no hay puerta trasera para un encargo incoherente.
- **El orden de declaración de `Sensitivity` es la política.** `Ord` sale de ahí y
  hay una prueba que lo fija.
- **`ProviderKind` sólo tiene `cli`.** Porque sólo `cli` está demostrado. Añadir
  `http` exigirá un manifiesto y un canario que lo ejerzan (R2).


## El hallazgo que más cambió el diseño

Al medir el transporte apareció esto, y merece contarse porque es una clase de fallo, no
una anécdota:

Para fijar qué modelo ejecuta un encargo, la vía documentada era una capa de parche. Se
aplicó, y la herramienta de inspección confirmó que el árbol de configuración había
cambiado. **La corrida fue a otro modelo igualmente.** Tres veces. Lo delataba el registro
de sesión, no la inspección: por encima de la capa de parche había un documento de ajustes
del usuario que ganaba en silencio.

Dos consecuencias que están en el código:

1. **El recibo anota lo observado, nunca lo pedido.** Es el vocabulario
   `ProvenanceSource`: `session_log` es una medición y `declared` una promesa. Un recibo
   que anota lo pedido miente justo sobre lo que le da valor.
2. **Una comprobación estática que enseña tu propio valor puesto no es evidencia.** Es la
   misma trampa por la que un canario de este proyecto devolvía «sin cuota» en 126 ms sin
   tocar la red: leía el fichero de política que él mismo debía informar.

## Sobre delegar

Los cuerpos de `batuta-manifest` no los escribió una persona: los escribió un modelo
externo, en un worktree aislado, con el modelo fijado y la contención declarada, y el
resultado se revisó línea a línea antes de integrarlo. Está medido en
`docs/medidas/DELEGACION_MANIFEST.md`.

Lo que hizo que funcionara no fue el modelo: fueron **los tests escritos antes**, que
fijaban los mensajes y no sólo los tipos. Lo único que salió mal —un `deny_unknown_fields`
que faltaba, y que dejaba pasar en silencio un campo mal escrito— es exactamente lo único
que no estaba cubierto por un test.

Es la misma regla del proyecto, aplicada a su propia construcción: **nada se declara, se
demuestra.**
