# Registro de deuda modular Rust

Estado: registro vivo.  
Última revisión: 2026-08-31.

Este documento aplica las señales de diseño de
[`REGLAS_INGENIERIA_RUST.md`](REGLAS_INGENIERIA_RUST.md) al árbol actual. El número de
líneas no demuestra por sí solo un defecto: señala dónde revisar cohesión, motivos de cambio y
coste de navegación. Los conteos son líneas físicas después de `cargo fmt`.

## Divisiones cerradas en K4

| Frontera | Resultado | Módulos resultantes |
|---|---|---|
| Coordinación de runs | El archivo raíz sólo ensambla y reexporta; intento, recuperación, modelo, runtime, servicio y soporte tienen fronteras separadas | 18–411 líneas por módulo |
| Parseo de CLI | La gramática heredada, operativa y de routing ya no comparte un único flujo | 174–341 líneas por módulo |
| Binario CLI | `main.rs` despacha y los handlers se agrupan por capacidad | 14–245 líneas por módulo |
| `RunReceiptV2` | Contrato, validación criptográfica y persistencia append-only están separados | 61–274 líneas por módulo |
| TUI `Execution` | Estado operativo, interacción, render/teclas, presentación y worker están separados | 61–366 líneas por módulo |

Estas divisiones preservan la API pública y están cubiertas por las suites de coordinador,
recibos, argumentos, superficies CLI/TUI y concurrencia multiproceso.

## Señales pendientes

| Archivo | Líneas | Responsabilidades observadas | Próxima extracción segura | Pruebas de caracterización |
|---|---:|---|---|---|
| `crates/batuta-manifest/src/manifest.rs` | 1282 | contratos, deserialización, validación semántica, resolución del ejecutable y accessors | `manifest/model.rs`, `parse.rs`, `validation.rs` y `executable.rs`, manteniendo las reexportaciones actuales | `batuta-manifest/tests/carga.rs` y `hash_manifest.rs` |
| `crates/batuta-routing/src/selector.rs` | 883 | contratos públicos, descartes serializables, elegibilidad, puntaje y construcción de la decisión | `selector/model.rs`, `discard.rs` y `engine.rs` | `selector.rs`, `public_discards.rs`, `sealed_decision.rs` y `application_service.rs` |
| `crates/batuta-quality/src/model.rs` | 557 | observaciones, migración V1, perfiles, overrides y errores | `model/observation.rs`, `profile.rs` y `override_event.rs` | `calidad.rs` e `initial_profiles.rs` |
| `crates/batuta-routing/src/operational.rs` | 546 | propuestas de investigación y recibos de canarios de capacidades | `operational/research.rs`, `canary.rs` y validadores privados compartidos sólo si representan la misma regla | `operational_v2.rs` y `research.rs` |
| `crates/batuta-contract/src/ids.rs` | 538 | varias familias de identificadores y rutas relativas bajo `no_std` | separar por familia semántica, conservando constructores, serde y mensajes byte a byte | `identificadores.rs` y `orden_y_serde.rs` |

La prioridad normal es `manifest.rs` y `selector.rs`, porque combinan más motivos de cambio y
superan ampliamente la señal de 500 líneas. Los demás se dividen al tocarlos de forma
sustancial o antes si una modificación cruza dos de sus responsabilidades.

## Procedimiento para saldar una entrada

1. Fijar primero la conducta con las pruebas de caracterización indicadas.
2. Mover una responsabilidad completa, no bloques elegidos sólo por cantidad de líneas.
3. Mantener privados los detalles nuevos y conservar la API pública mediante reexportaciones.
4. No mezclar el movimiento mecánico con cambios de conducta salvo que una prueba roja lo exija.
5. Ejecutar formato, Clippy, tests del crate y `scripts_ci/local_gates.sh`.
6. Actualizar este registro con los módulos finales y retirar la entrada sólo después de los
   gates verdes.
