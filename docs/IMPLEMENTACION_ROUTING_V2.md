# Estado de implementación: calidad y routing v2

> **Guía de orientación, no autoridad de conducta.** El estado futuro se navega desde
> [`ROADMAP.md`](../ROADMAP.md) y el contrato vigente desde [`specs/system/`](../specs/system/). Esta
> guía conserva contexto de implementación y no sustituye ninguna spec viva.

Fecha de verificación: 2026-08-31.

Este documento separa código verificable de mediciones que necesitan ejecutar
proveedores externos. Un esquema o un dato del fabricante nunca se presenta como
evidencia de una ruta exacta.

## Implementado y cubierto por tests

- SPEC de ejecución, política, calidad, panel, CLI, MCP y TUI.
- `RouteRef` exacta `harness/proveedor/modelo[/revisión]`; MiniMax y OpenCode
  siguen siendo rutas DSH.
- Observaciones crudas, cestas por acción, cobertura, caducidad, compatibilidad,
  configuración, scaffold, métrica, revisión, rango contribuyente, fuente,
  descartes estructurados y overrides append-only `set`/`clear`.
- Hash de calidad canónico sobre ruta/revisión, perfil, contribuciones,
  exclusiones e historial; reordenar entradas no cambia resultado ni sello.
- `StateManifestV2` generacional con catálogo, política, evidencia, salud e
  índice de capacidades guardados como objetos JSON inmutables por contenido.
- Ensamblado tipado desde una única generación, exactamente un candidato por
  ruta/acción y sin defaults ocultos; la decisión sella los cinco componentes y
  los recibos usados.
- Commit de estado que sincroniza objetos antes del rename atómico del manifest;
  un fallo conserva activa la generación anterior y cualquier alteración se
  detecta por hash antes de ensamblar.
- Lease de escritor y CAS contra el manifest base para rechazar publicaciones
  obsoletas entre procesos.
- Staging de investigación con hash, confirmación, conflicto de base, persistencia
  atómica e independencia de la ruta investigadora.
- Selector determinista por capacidad, sensibilidad, contexto, esfuerzo, clase,
  cooldown, calidad verificada, margen, fallback aprobado y coste esperado.
- Salud durable, `Retry-After`, sondas 15/30/60 minutos y horarias para MiniMax,
  bloqueo por autenticación/saldo y relevo mediante `HandoffCheckpoint`.
- Frontera `RouteRequestV2`: el cliente no puede suministrar candidatos, perfiles,
  puntajes ni hashes. `ApplicationService` fija una foto local v2 para CLI/MCP/TUI.
- `catalog import/status/apply`: normalización DSH, staging sellado, conflicto de
  base y confirmación. OpenCode requiere sus cuatro costes conocidos, finitos y
  cero; todas las rutas importadas nacen en `probe/test`.
- Sidecar DSH JSONL que usa sólo `listProviders`, `listModels` y
  `resolveModelInfo`, con `stream` prohibido, redacción, costes desconocidos,
  entorno allowlisted, timeout, cierre del árbol y salida acotada.
- Persistencia atómica de snapshots derivados y ejecución; migración de política
  v1→v2 con dry-run, diff, backup recuperable y segunda aplicación idempotente.
- `RoutingReceipt` append-only con petición, proyecciones, decisión, hashes,
  transiciones y checkpoint. Una compuerta concurrente impide dos invocaciones.
- Tabla, HTML y TUI muestran ruta, investigado, override, efectivo, cobertura,
  verificación y hashes desde la misma `RouteDecision`.
- Los ejemplos JSON válidos e inválidos del SPEC se validan automáticamente.
- `ExecutionGrantV1`, revocaciones append-only, ledger durable de cuatro
  dimensiones y coordinador que sincroniza journal antes de una única invocación.
- Recuperación de `invocation_started` como `outcome_unknown`, sin reenvío.
- `ExecutionProfileV1` cerrado y sellado, con staging, diff, CAS y aplicación
  confirmada compartida por CLI, TUI y ejecución.
- Ejecutor que resuelve ruta, pin, hash, argv, materialización y entorno únicamente
  desde manifests confiables; cualquier fallo desconocido es permanente.
- Ventana de salud durable de veinte observaciones, ambiguos conservadores, p95 por
  rango más próximo y publicación CAS sin perder escrituras concurrentes.
- Retry exclusivo para rate limit con `Retry-After`, reserva atómica de espera e
  intento y fallback limitado a rutas no intentadas que siguen en el grant.
- `HandoffCheckpoint` sin historial y `RunReceiptV2` exhaustivo, sellado,
  append-only y estable byte a byte tras reinicio.
- CLI operativa `executor profile`, `grant` y `run`, siempre bajo `ApiResponseV2`
  o un `ApiErrorV2` cerrado.
- Vista TUI `Execution` operable por teclado para perfil, grants y runs; admite
  formularios y JSON pegado, muestra diff o preview sin reserva, exige confirmación
  escrita y usa un único worker de fondo.
- Exclusión multiproceso sobre el mismo run/grant: como máximo una invocación y
  ningún sobreconsumo de presupuesto.
- Descartes públicos `{code, field, message, details}` y sobre `ApiResponseV2`
  común a CLI, MCP y TUI; MCP no acepta rutas de almacenamiento.
- `ResearchProposalV2` con fuentes completas, independencia y conflicto triple.
- `ToolEventV2` y recibos de canario que exigen efectos exactos para lectura,
  escritura, herramientas y web.

## Pendiente de operación externa

- V1: recopilar evidencia primaria actual para rutas realmente disponibles.
- V2: ejecutar benchmarks locales por acción y ruta exacta.
- V3: comparar tokens, calidad, fallos y coste de relevo contra el baseline.
- Ejecutar canarios de capacidad sobre cada ruta real.

Estas tareas pueden consumir cuota o depender de autenticación del harness. No se
ejecutaron: Batuta no leyó credenciales, saldos ni suscripciones.

## Límites conocidos

Faltan la propuesta y CLI transaccional de política, la migración completa del
estado v1 y hacer que todas las aplicaciones publiquen mediante el mismo CAS.

La TUI operativa K4 está cerrada. El CRUD interactivo de catálogo, alias, costes,
cestas, fallbacks y overrides queda fuera de este bloque y sigue pendiente.

Tampoco se declara terminado el ejecutor síncrono de `research update`: el flujo
valida propuesta v2, fuentes, conflictos e independencia, pero falta conectar
selección, grant research, fake/harness y publicación mediante `StateStore`.

Las órdenes v1 (`enable`, `disable`, `effort`, `panel`, alta/baja de proveedor y
modelo) siguen visibles; el corte limpio K7.6 aún no se ha ejecutado.

## Verificación

```text
bash scripts_ci/local_gates.sh
  formato: OK
  no_std del contrato: OK
  evidencia TDD JSONL: OK
  clippy -D warnings: OK
  cargo test --workspace --all-features: OK (suite ampliada; recuento en el gate)
```

Mutaciones dirigidas observadas y restauradas:

- margen de selección: detectada por el test del selector;
- revisión opcional de `RouteRef`: detectada por `route_ref`;
- filtro OpenCode de `all` a `any`: detectada (5 rutas frente a 3);
- compuerta serial desactivada: detectada por el test concurrente;
- orden de hashes de recibos invertido: detectado por `sealed_decision`;
- campo público de descarte alterado: detectado por `public_discards`;
- validación de límite cero desactivada: detectada por `grants`;
- conflicto de manifest y evento exitoso invertidos: detectados por `operational_v2`;
- versión del sobre público alterada: detectada por `routing_surfaces`.
- intersección del grant omitida: detectada por `coordinator_v2`;
- `invocation_started` no sincronizado antes del ejecutor: detectada por
  `coordinator_acceptance`;
- límite de retry invertido: detectado por `coordinator_acceptance`;
- p95 sustituido por el máximo: detectado por `health_window`;
- historial original reenviado al fallback: detectado por `coordinator_acceptance`;
- recuperación ambigua convertida en reintento: detectada por `coordinator_v2`;
- confirmación TUI del run omitida: detectada por `tui_execution`.
- atajo de ejecución TUI retirado: detectado por el test unitario del mapa de teclas.
