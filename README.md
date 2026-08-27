# batuta

Orquestador de delegación en Rust. **Añadir un proveedor es un fichero, nunca un parche.**

El núcleo no conoce ningún proveedor: los lee de manifiestos declarativos releídos en
cada invocación.

Nació de un fallo concreto y medido: en el orquestador anterior, el transporte de un
proveedor estaba **declarado en un registro y ausente del otro**, así que toda tarea
enrutada allí moría *después* de pagar la corrida. De ahí sale la regla que ordena todo lo
demás: **nada se declara, se demuestra.**

- `docs/ESQUEMA_MANIFIESTO.md` — el esquema de manifiestos y por qué cada campo existe.
- `docs/medidas/DSH_HEADLESS.md` — las mediciones sobre las que se apoya el diseño.

## Estado

| Fase | Qué | Estado |
|---|---|---|
| **0** | **Desbloqueo y medición del transporte** | **hecho** (`docs/medidas/`) |
| **1** | **Workspace, `batuta-contract`, gates** | **hecho** |
| **2** | **Manifiestos** | **en curso — fase roja de TDD** |
| 3 | Ejecución, plugins, recibos, leases | pendiente |
| 4 | Política y benchmark | pendiente |
| 5 | Superficies MCP y CLI | pendiente |
| 6 | Convivencia y corte | pendiente |

El primer beneficio real llega en la Fase 3, no al final.

## Qué hay hoy

```
Cargo.toml                       workspace
crates/batuta-contract/          tipos, errores y vocabularios cerrados. CERO E/S
crates/batuta-manifest/          carga y validación de manifiestos  [firmas + tests en rojo]
providers/dsh.toml               DeepSeek Harness
providers/abacus.toml            Abacus.AI — el proveedor que originó el proyecto
docs/ESQUEMA_MANIFIESTO.md       el esquema y su justificación
docs/medidas/DSH_HEADLESS.md     lo que se midió del transporte, con las corridas
scripts_ci/local_gates.sh        los gates permanentes
.github/workflows/ci.yml         los mismos gates en CI
```

`batuta-contract` no depende de ningún otro crate de batuta y todos dependerán de él.

**`batuta-manifest` está hoy en la fase roja de TDD**, y está declarado: las firmas
públicas existen, los catorce tests existen y fallan, y los cuerpos son `todo!()`. Los
tests fijan los *mensajes* de error, no sólo los tipos —el que rechaza un `parser`
inválido exige que el mensaje liste los cuatro valores válidos—, porque un error que no
enumera lo que valía es el fallo que R8 paga.

## Gates

```sh
bash scripts_ci/local_gates.sh
```

Cuatro, y ninguno es opcional:

1. `cargo fmt --all --check`
2. `batuta-contract` sigue siendo `no_std`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`

Prioridad baja y dos jobs por defecto, como manda `AGENTS.md` de CHUNSA001.
`BATUTA_BUILD_JOBS` es un override consciente.

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
