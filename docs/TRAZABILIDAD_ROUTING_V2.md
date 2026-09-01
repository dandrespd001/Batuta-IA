# Trazabilidad de cierre de routing v2

> **Evidencia histórica, no autoridad de conducta.** La navegación vigente comienza en
> [`specs/anchors.json`](../specs/anchors.json) y la evidencia sellada permanece en
> [`docs/evidence/`](evidence/). Este documento demuestra el cierre que registró y no sustituye las
> specs vivas.

Fecha de auditoría: 2026-08-31. `OK` significa cubierto por código y test local;
`EXTERNO` requiere autorización, límites y acceso a un harness real; `PENDIENTE`
no debe interpretarse como terminado.

| Parte | Estado | Evidencia principal |
|---|---|---|
| A — contrato y baseline | OK | `route_ref`, `routing_surfaces`, ejemplos JSON y `local_gates.sh` |
| B — calidad | OK | Proyecciones tipadas consumidas por el ensamblador de una generación |
| C — estado, DSH y OpenCode | PARCIAL | Ensamblado tipado, CAS y sidecar offline verdes; faltan propuesta/aplicación unificada de política y migración del estado v1 completo |
| D — selector | OK | Frontera estricta, descartes públicos y sello de manifest/componentes/recibos cubiertos |
| E — ejecución y recibos | OK | Perfil, grant, ledger, journal, salud de 20, retry, relevo, recibo final, recuperación ambigua y exclusión multiproceso verdes |
| F — investigación controlada | PARCIAL | Propuesta v2, fuentes, conflicto triple e independencia verdes; falta orquestación síncrona completa con investigador seleccionado |
| G — superficies K4 | OK | Comandos `executor profile`, `grant` y `run`, vista TUI Execution, paridad y sobres uniformes verdes; el corte v1 pertenece a K7.6 |
| H — V1–V3 y canarios reales | EXTERNO | sólo dobles locales; no se gastó cuota ni se accedió a credenciales |
| I — auditoría y rollout | EXTERNO | gates y mutaciones verdes; promoción real queda bloqueada hasta V1–V3 |

## Correspondencia de aceptación

| Requisito | Test o artefacto |
|---|---|
| Puntaje por acción sin mutar observaciones | `batuta-quality/tests/calidad.rs` |
| Incompatibilidad, cobertura, caducidad y confianza | `batuta-quality/tests/calidad.rs` |
| Override auditado | `batuta-quality/tests/calidad.rs`, `routing_surfaces.rs` |
| Propuesta nunca autoaplicada | `research_store.rs`, `catalog_store.rs` |
| Cliente no inyecta candidatos ni hashes | `batuta-routing/tests/application_service.rs` |
| Margen, coste, desempates y autorizaciones separadas | `batuta-routing/tests/selector.rs` |
| OpenCode sólo DSH y coste cero demostrado | `batuta-routing/tests/catalog.rs` |
| Una sola invocación activa | `batuta-routing/tests/serial_executor.rs` |
| Retry, cooldown, fallback y recuperación | `health_handoff.rs`, `run_state.rs`, `run_store.rs`, `coordinator_acceptance.rs` |
| Ventana exacta de salud y CAS concurrente | `health_window.rs` |
| Recibo con hashes y checkpoint | `routing_receipt.rs`, `run_receipt_v2.rs` |
| Misma decisión en superficies | `batuta-cli/tests/routing_surfaces.rs` |
| Migración recuperable e idempotente | `policy_migration.rs` (sólo política; el estado v1 completo sigue pendiente) |
| Compatibilidad completa y revisiones no mezclables | `batuta-quality/tests/calidad.rs` |
| Descartes, antigüedad y fuente reproducibles | `batuta-quality/tests/calidad.rs` |
| Overrides append-only `set`/`clear` | `batuta-quality/tests/calidad.rs` |
| Hash de calidad estable ante reordenación | `batuta-quality/tests/calidad.rs` |
| Objetos inmutables y manifest transaccional | `batuta-routing/tests/state_store.rs` |
| CAS entre escritores con la misma base | `batuta-routing/tests/state_store.rs` |
| Ensamblado tipado desde una sola generación | `batuta-routing/tests/state_assembly.rs` |
| Sidecar DSH sin `stream` ni secretos | `sidecar/test_dsh_catalog.mjs`, `batuta-routing/tests/dsh_sidecar.rs` |
| Manipulación de un objeto detectada por hash | `batuta-routing/tests/state_store.rs` |
| JSON público sin reloj, clase ni fallback | `batuta-routing/tests/application_service.rs` |
| Autorización solicitada no se autoconcede | `batuta-routing/tests/request_profile.rs` |
| Descartes públicos cerrados | `batuta-routing/tests/public_discards.rs` |
| Decisión sellada y bytes deterministas | `batuta-routing/tests/sealed_decision.rs` |
| Grant, revocación y presupuesto durable | `batuta-routing/tests/grants.rs` |
| Journal previo y crash ambiguo sin reenvío | `batuta-routing/tests/coordinator.rs` |
| Reserva y journal visibles antes del ejecutor | `batuta-routing/tests/coordinator_acceptance.rs` |
| Una invocación máxima entre procesos | `batuta-routing/tests/coordinator_multiprocess.rs` |
| Investigación v2 y fuentes exactas | `batuta-routing/tests/operational_v2.rs` |
| Canarios por efectos externos exactos | `batuta-routing/tests/operational_v2.rs` |
| Sobre uniforme y MCP sin rutas arbitrarias | `batuta-cli/tests/routing_surfaces.rs` |
| Perfil, grants y runs por API/CLI | `batuta-cli/tests/operational_api.rs`, `operational_args.rs` |
| Paridad CLI, formulario TUI, JSON y controles alcanzables | `batuta-cli/tests/tui_execution.rs`, test unitario `tui::terminal::tests` |

## Evidencia TDD y mutación

Los nuevos contratos se ejecutaron primero en rojo: símbolos ausentes para
servicio, catálogo, stores, recibo, compuerta y migración; rechazo de revisión;
autocertificación aceptada; y divergencia de superficies. Tras la implementación
quedaron verdes. Las mutaciones restauradas están detalladas en
`IMPLEMENTACION_ROUTING_V2.md`.

## Condición de finalización

La vertical operativa K4 se declara cerrada con ejecutables falsos y sin consumo de
cuota. El programa completo de routing v2 continúa abierto mientras C, F, H e I no
pasen a `OK`. En especial, ninguna ruta debe promoverse antes de V1–V3 y canarios
exactos con una confirmación nueva que fije solicitudes, tokens y tiempo. Esa
autorización no se infirió.

## Diferido conscientemente

- CRUD TUI completo de catálogo, perfiles, cestas, fallbacks, overrides y propuestas.
- Evidencia primaria V1 para todas las rutas disponibles.
- Benchmarks locales V2 por ruta y acción.
- Comparación V3 general contra baseline.
- Rollout y promoción integral de todas las rutas.

Hasta completar lo anterior, el resultado admisible es «vertical operativa K4
cerrada»; nunca «Batuta terminada» ni «routing v2 promovido».
