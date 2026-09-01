# Producto y contratos vigentes

Esta spec es autoridad viva para el vocabulario público y la frontera de contratos de dominio. La
evidencia inicial enlazada desde anchors conserva el `evidence_mode` de cada fila K4; no se presenta
ningún `reconstructed_audit` como TDD retroactivo.

## CAP-CONTRACTS — Contratos de dominio cerrados

**Estado**: `implemented`. **Roadmap**: `RM-002` conserva una evolución interna pendiente sin cambiar
el estado funcional.

### REQ-CONTRACTS-001 — Identidad y vocabulario cerrados

IDs, rutas, tareas, acciones, sensibilidad y demás vocabularios públicos son tipos cerrados,
versionados cuando se persisten, rechazan formas desconocidas y serializan de manera determinista.

### REQ-CONTRACTS-002 — Frontera de dominio sin E/S

`batuta-contract` permanece `no_std`, no accede a ficheros, red, reloj ni procesos y no depende de
otro crate interno; las capas con efectos convierten sus datos en los contratos cerrados.

La verificación ejecutable y la evidencia histórica se mantienen en `specs/anchors.json`.
