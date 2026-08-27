# batuta

Orquestador de delegación en Rust. **Añadir un proveedor es un fichero, nunca un parche.**

El núcleo no conoce ningún proveedor: los lee de manifiestos declarativos releídos en
cada invocación. Diseño y motivación en
`../CHUNSA001/docs/briefs/BATUTA_ARRANQUE.md`.

## Estado

| Fase | Qué | Estado |
|---|---|---|
| 0 | Desbloqueo de DeepSeek y retirada de la clave en claro | pendiente, fuera de este repo |
| **1** | **Workspace, `batuta-contract`, gates** | **hecho** |
| 2 | Manifiestos y credenciales | pendiente |
| 3 | Ejecución, plugins, recibos, leases | pendiente |
| 4 | Política y benchmark | pendiente |
| 5 | Superficies MCP y CLI | pendiente |
| 6 | Convivencia y corte | pendiente |

El primer beneficio real llega en la Fase 3, no al final.

## Qué hay hoy

```
Cargo.toml                       workspace
crates/batuta-contract/          tipos, errores y vocabularios cerrados. CERO E/S
scripts_ci/local_gates.sh        los gates permanentes
.github/workflows/ci.yml         los mismos gates en CI
```

`batuta-contract` no depende de ningún otro crate de batuta y todos dependerán de él.

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
