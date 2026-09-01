# Rollout y validación externa

## CAP-ROLLOUT — Evidencia externa y canarios reales

**Estado**: `external`. **Roadmap**: `RM-007`. Ninguna actividad de esta capacidad está autorizada por
el paquete 001.

### REQ-ROLLOUT-001 — Evidencia primaria de proveedores

Cada afirmación operativa sobre rutas y proveedores debe apoyarse en fuente primaria o medición real
sellada con ruta, revisión, escenario, configuración, fecha y procedencia.

### REQ-ROLLOUT-002 — Benchmarks V2 reproducibles

Las campañas V2 deben ejecutar escenarios, límites y métricas aprobados, conservar datos brutos y
normalizados y demostrar compatibilidad antes de incorporar una observación.

### REQ-ROLLOUT-003 — Comparación V3

La comparación V3 debe usar la misma cesta, configuración, ventanas y criterios para todas las rutas,
publicar exclusiones y no promover una conclusión con cobertura insuficiente.

### REQ-ROLLOUT-004 — Canarios reales

Los canarios reales deben verificar efectos exactos —no menciones plausibles—, sellar manifest,
grant, ruta, límites, eventos y vencimiento, y ejecutar únicamente dentro de una autorización humana
separada con presupuesto y rollback.

## Protocolo manual para REQ-ROLLOUT-001..REQ-ROLLOUT-004

Precondiciones: paquete futuro de `RM-007` y autorización vigente que enumere alcance, rutas, límites,
presupuesto, caducidad y rollback. Registrar la fuente primaria; ejecutar V2; producir V3 sobre el
mismo conjunto; ejecutar los canarios de lectura, escritura, herramientas y web; revocar la
autorización al terminar. Observar artefactos, costes, recibos y ausencia de rutas laterales. Se acepta
sólo si cada requisito tiene evidencia sellada propia, todos los efectos caben en los límites y el
rollback deja el estado previo verificable. Sin autorización, el resultado correcto es no ejecutar.
