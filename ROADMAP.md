# Roadmap de Batuta

Las entradas conservan su ID aunque cambien de título. `package` sólo deja de ser `null` cuando el
directorio existe y enlaza de vuelta a esta entrada. Ninguna autorización externa se presume por la
mera existencia de una capacidad o un grant técnico.

<a id="rm-001"></a>
## RM-001 — Adopción spec-anchored

- **Alcance**: publicar autoridad viva, anchors, clasificación documental y el validador estructural
  para las once capacidades actuales, sin modificar producto.
- **Dependencias**: baseline K4 `7de68af2c9a36ba3dcc65971e4bba83231fb3855` y aprobación humana del
  paquete 001.
- **Estado**: `in_progress`
- **Aceptación**: `CAP-CONTRACTS`, `CAP-EXECUTION`, `CAP-LEGACY`, `CAP-MANIFESTS`, `CAP-POLICY`,
  `CAP-QUALITY`, `CAP-RESEARCH`, `CAP-ROLLOUT`, `CAP-ROUTING`, `CAP-STATE` y `CAP-SURFACES` quedan
  navegables y la validación estructural acepta exactamente sus 24 requisitos.
- **Paquete**: `specs/001-adopt-spec-anchoring`
- **Autorización externa**: `null`

<a id="rm-002"></a>
## RM-002 — Dividir contratos y manifests heredados

- **Alcance**: dividir `crates/batuta-manifest/src/manifest.rs` y
  `crates/batuta-contract/src/ids.rs` sin alterar API, serialización, errores ni hashes.
- **Dependencias**: `RM-001` y caracterización vigente de ambos crates.
- **Estado**: `planned`
- **Aceptación**: `CAP-CONTRACTS` y `CAP-MANIFESTS` conservan `REQ-CONTRACTS-001`,
  `REQ-CONTRACTS-002`, `REQ-MANIFESTS-001` y `REQ-MANIFESTS-002` byte a byte donde corresponda;
  las dos excepciones modulares se retiran al quedar bajo umbral.
- **Paquete**: `null`
- **Autorización externa**: `null`

<a id="rm-003"></a>
## RM-003 — Dividir selección, calidad y contratos operativos

- **Alcance**: dividir `selector.rs`, `model.rs` y `operational.rs` por responsabilidad sin cambio
  funcional.
- **Dependencias**: `RM-001` y caracterización de decisiones, proyecciones y documentos operativos.
- **Estado**: `planned`
- **Aceptación**: `CAP-ROUTING` y `CAP-QUALITY` conservan `REQ-ROUTING-001`,
  `REQ-ROUTING-002`, `REQ-QUALITY-001` y `REQ-QUALITY-002`; las tres excepciones modulares se retiran.
- **Paquete**: `null`
- **Autorización externa**: `null`

<a id="rm-004"></a>
## RM-004 — Estado y política transaccionales

- **Alcance**: añadir propuesta/aplicación transaccional de política, migración completa V1 y una
  frontera CAS común para escritores de estado.
- **Dependencias**: `RM-001`, contratos persistidos vigentes y plan de migración recuperable.
- **Estado**: `planned`
- **Aceptación**: `REQ-STATE-002` y `REQ-POLICY-002` pasan a `implemented` con pruebas de staging,
  base obsoleta, backup, reintento idempotente y publicación atómica; `CAP-STATE` y `CAP-POLICY`
  derivan entonces su nuevo estado.
- **Paquete**: `null`
- **Autorización externa**: `null`

<a id="rm-005"></a>
## RM-005 — Research update síncrono

- **Alcance**: ejecutar de extremo a extremo `research update` de forma síncrona y dejar propuesta
  sellada en staging sin autocertificación.
- **Dependencias**: `RM-004` para publicación transaccional y un perfil web-capable aprobado.
- **Estado**: `planned`
- **Aceptación**: `REQ-RESEARCH-002` ejecuta consulta, normalización, sellado y staging en una misma
  operación observable, sin activar evidencia ni usar la ruta investigadora como fuente propia.
- **Paquete**: `null`
- **Autorización externa**: `null`

<a id="rm-006"></a>
## RM-006 — CRUD compartido entre superficies

- **Alcance**: exponer un CRUD de biblioteca común para catálogo, perfiles, cestas, fallbacks,
  overrides y propuestas, consumido por CLI, JSON, MCP y TUI.
- **Dependencias**: `RM-004` y `RM-005` para las mutaciones transaccionales que presenta.
- **Estado**: `planned`
- **Aceptación**: `REQ-SURFACES-002` pasa a `implemented`; las superficies producen el mismo estado,
  errores y confirmaciones para cada mutación y una cancelación no escribe.
- **Paquete**: `null`
- **Autorización externa**: `null`

<a id="rm-007"></a>
## RM-007 — Evidencia y canarios reales

- **Alcance**: obtener evidencia primaria, benchmarks V2, comparación V3 y canarios reales con
  límites y procedencia explícitos.
- **Dependencias**: autorización humana futura con alcance, rutas, límites, presupuesto, caducidad y
  rollback; disponibilidad de los sistemas externos.
- **Estado**: `external`
- **Aceptación**: `REQ-ROLLOUT-001`, `REQ-ROLLOUT-002`, `REQ-ROLLOUT-003` y `REQ-ROLLOUT-004` quedan
  satisfechos sólo por evidencia primaria sellada; `CAP-ROLLOUT` no cambia de estado antes.
- **Paquete**: `null`
- **Autorización externa**: `null`

<a id="rm-008"></a>
## RM-008 — Retirada de órdenes V1

- **Alcance**: retirar limpiamente las órdenes V1 tras una ventana de deprecación y una ruta de
  migración verificable hacia las superficies V2.
- **Dependencias**: `RM-006`, inventario de consumidores y aprobación explícita de compatibilidad.
- **Estado**: `planned`
- **Aceptación**: `REQ-LEGACY-001` conserva compatibilidad hasta el corte publicado; después las
  órdenes fallan con migración accionable y ningún dato histórico se reescribe.
- **Paquete**: `null`
- **Autorización externa**: `null`
