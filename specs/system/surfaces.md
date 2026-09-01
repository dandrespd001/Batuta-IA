# Superficies públicas y compatibilidad vigentes

## CAP-LEGACY — Órdenes V1 de compatibilidad

**Estado**: `deprecated`. **Roadmap**: `RM-008`.

### REQ-LEGACY-001 — Compatibilidad V1 durante la deprecación

Las órdenes V1 existentes continúan parseando y ejecutando su contrato conocido durante la ventana
de deprecación, pero no reciben capacidades nuevas. Su retirada exige inventario de consumidores,
migración accionable y corte aprobado en `RM-008`.

## CAP-SURFACES — CLI, JSON, MCP y TUI

**Estado**: `partial`. **Roadmap**: `RM-006`.

### REQ-SURFACES-001 — Paridad de consulta y decisión

CLI humana/JSON, MCP por stdio, TUI, tabla y HTML consumen la misma aplicación y presentan la misma
decisión, puntajes, hashes, autorizaciones y errores cerrados. Una simulación no ejecuta ni reserva y
MCP no abre sockets ni expone aplicación de parches.

Las órdenes operativas vigentes incluyen grants, run/status/resume y perfil
import/status/apply. Toda respuesta exitosa usa un sobre versionado y todo error expone exactamente
versión, código, campo, mensaje y detalles. MCP usa JSON-RPC 2.0 con un objeto por línea; la TUI
distingue catálogo, política, evidencia/staging y salud/recibos sin servidor.

### REQ-SURFACES-002 — CRUD compartido

**Estado parcial**: existen operaciones y flujos confirmados por superficies concretas, pero falta un
CRUD común y completo para catálogo, perfiles, cestas, fallbacks, overrides y propuestas consumido
con paridad por CLI, JSON, MCP y TUI. La aceptación pertenece a `RM-006`.

## Protocolo manual para REQ-SURFACES-002

Precondiciones: paquete de `RM-006`, el mismo fixture inicial y todas las superficies offline.
Crear, consultar, actualizar, cancelar y eliminar cada tipo soportado desde cada superficie; comparar
estado canónico, errores, diff y confirmación. Se acepta si las operaciones equivalentes producen el
mismo documento y una cancelación o fallo conserva exactamente el activo anterior.
