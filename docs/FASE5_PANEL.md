# SPEC v2 — Superficies CLI, TUI y MCP

> **Archivo histórico desde 2026-08-31.** La autoridad vigente es
> [`specs/system/surfaces.md`](../specs/system/surfaces.md). La matriz completa de requisitos y
> verificaciones está en [`DOCUMENT_CLASSIFICATION.md`](DOCUMENT_CLASSIFICATION.md); el contenido
> siguiente se conserva íntegro.

**Estado: aprobado para implementación.** Este documento reemplaza la propuesta de Fase 5.

## 1. Una sola aplicación, tres superficies

Las operaciones de catálogo, política, calidad, investigación y selección son funciones
puras de biblioteca. La tabla, el HTML, la CLI JSON, el MCP y la TUI no reimplementan sus
reglas; únicamente convierten entrada y presentan salida.

Las superficies son:

- CLI humana y CLI JSON versionada, leyendo JSON desde `--json`, `--file` o stdin;
- MCP JSON-RPC 2.0 por stdio, sin servidor de red;
- `batuta tui`, con `ratatui`/`crossterm`, además de la tabla y HTML existentes.

## 2. Operaciones

La TUI y la CLI permiten importar y confirmar rutas de DSH; resolver alias; añadir,
quitar, habilitar o deshabilitar harnesses y modelos; editar esfuerzos y costes; crear
perfiles; editar cestas, calidad mínima, cobertura, antigüedad y margen; revisar evidencia,
override y puntaje efectivo; configurar fallbacks y `allow_any_eligible`; actualizar y
aplicar investigación; consultar cooldown, procedencia y recibos.

Cambios persistentes se guardan de forma atómica: escribir un temporal en el mismo
directorio, sincronizar, renombrar y sincronizar el directorio. Un fallo conserva el fichero
anterior completo.

## 3. CLI JSON

```
batuta route --json '{...}'
batuta route --file request.json
printf '%s' '{...}' | batuta route
batuta research update --all
batuta research status
batuta research apply <id> --confirm
batuta tui
batuta mcp
```

Toda respuesta JSON tiene `schema_version`. Errores de contrato salen como objetos con
`code`, `message` y `details`, y código de proceso 2. Una simulación de ruta no ejecuta ni
gasta.

## 4. MCP por stdio

El método `tools/list` expone al menos `batuta.route`, `batuta.research.status` y
`batuta.catalog`. `tools/call` valida el mismo JSON que la CLI y llama al mismo selector.
El transporte usa un objeto JSON por línea; no abre sockets, no expone una operación que
acepte/aplique parches y no guarda historial de conversación.

## 5. TUI

La TUI funciona sin servidor. Vistas mínimas:

1. catálogo de harnesses/rutas/alias;
2. perfiles y política;
3. evidencia, override e investigación en staging;
4. salud, cooldown y recibos de routing.

La barra de estado siempre distingue puntaje investigado, override, efectivo, cobertura y
verificación. “Actualizar investigación” sólo crea staging; “Aplicar” exige confirmación.

## 6. Invariantes de interfaz

- Tabla, HTML, CLI, MCP y TUI muestran la misma `RouteDecision` y el mismo puntaje.
- Omitir `minimum_quality` o `selection_margin` usa el perfil de acción; un campo presente
  nunca se sustituye silenciosamente.
- `allow_any_eligible` y `allow_unverified_quality` se muestran de forma visible en la
  petición y en el recibo.
- Ninguna inspección toma el lease de una ejecución.
- Ninguna superficie toca credenciales o consulta saldo.

## 7. Aceptación

- El mismo JSON por CLI y MCP produce bytes equivalentes al normalizar el sobre MCP.
- Una TUI cerrada sin guardar no muta estado; una confirmación usa guardado atómico.
- Research `update` nunca altera la evidencia activa.
- La acción de aplicar rechaza una propuesta no confirmada o cuyo hash cambió.
- La ayuda enumera todas las órdenes y banderas admitidas.
