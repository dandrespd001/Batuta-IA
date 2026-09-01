# Esquema contractual — Calidad, research y routing v2

> **Archivo histórico desde 2026-08-31.** Las autoridades vigentes son
> [`specs/system/quality-research.md`](../specs/system/quality-research.md) y
> [`specs/system/state-policy-routing.md`](../specs/system/state-policy-routing.md). La matriz completa
> vive en [`DOCUMENT_CLASSIFICATION.md`](DOCUMENT_CLASSIFICATION.md); lo que sigue se conserva íntegro
> como contexto histórico.

Este documento es la referencia serializable de los SPEC v2.

## Identificadores

- `RouteRef`: `harness/provider/model[/revision]`, tres o cuatro segmentos no vacíos,
  sin `.` ni `..`. La revisión, cuando existe, forma parte de la identidad exacta.
- `ActionId`, `BenchmarkId`, `ObservationId`, `ProposalId`: texto no vacío de hasta 128
  bytes, compuesto por ASCII alfanumérico, `.`, `_` o `-`.
- Alias: la misma regla; cada alias resuelve a exactamente una `RouteRef`.

## Versiones

| Documento | `schema_version` |
|---|---:|
| política persistida | 2 |
| observación de benchmark legada | 1 (sólo lectura hasta migración explícita) |
| evidencia/propuesta v2 | 2 |
| petición/decisión de ruta | 2 |
| manifest de estado | 2 |
| grant de ejecución | 1 |
| petición/estado de ejecución | 2 |
| recibo de canario de capacidad | 2 |
| checkpoint | 1 |
| MCP | JSON-RPC 2.0 |

Una versión desconocida se rechaza antes de usar cualquier campo.

## Fechas y puntajes

Las fechas se expresan como segundos Unix UTC para comparaciones deterministas. Puntajes,
cobertura, márgenes y pesos están en `0..100`. Los pesos usan enteros y suman 100; los
puntajes pueden tener decimales finitos y deben ser números JSON, nunca `NaN` o infinito.

## Compatibilidad de una cesta de calidad

Cada componente fija por separado `benchmark`, `benchmark_version`, `scenario`,
`configuration`, `scaffold`, `metric` y, cuando se conoce, la revisión esperada.
Una observación sólo contribuye si coincide en todos ellos y en la `RouteRef`
exacta. Si la ruta declara revisión, la observación debe declarar la misma. Si la
ruta no la declara y existen varias revisiones compatibles con un componente, ese
componente entero queda inutilizable: Batuta nunca las promedia.

Cada observación considerada produce una contribución o una exclusión. Ambas hacen
visibles URL, tipo de fuente, fecha observada, edad y fecha de caducidad. Las
exclusiones usan códigos cerrados: `route_mismatch`, `benchmark_mismatch`,
`benchmark_version_mismatch`, `scenario_mismatch`, `configuration_mismatch`,
`scaffold_mismatch`, `metric_mismatch`, `revision_mismatch`,
`ambiguous_revision` y `expired`.

Los overrides son eventos append-only `set`/`clear`, ordenados por instante e
identificador. Un `clear` recupera el puntaje investigado sin borrar los `set`
anteriores. Ni un override ni evidencia exclusivamente del fabricante pueden
convertir una proyección en verificada.

## Frontera pública de routing

`RouteRequestV2` sólo contiene intención: acción, capacidades, sensibilidad,
contexto, esfuerzo, tokens, umbral/margen opcionales y autorizaciones solicitadas.
No admite candidatos, perfiles, hashes, reloj, clase de ejecución ni condición de
fallback. Esos valores proceden del servicio y del `StateManifestV2` activo.

`RouteDecisionV2` sella la ruta elegida, calidad, cobertura, coste, descartes
estructurados, autorizaciones efectivas y hashes de manifest, catálogo, política,
evidencia, salud e índice de capacidades. El reloj usado es `now`, fijado por el
servicio; nunca una fecha suministrada por el cliente.

OpenCode sólo existe bajo `dsh/opencode/<modelo>`. La clase
`production|probe_test` y la condición de fallback son datos internos y quedan
fuera de la entrada pública.

## Estado v2

`StateManifestV2` contiene `schema_version = 2`, una generación monótona y los
hashes SHA-256 de catálogo, política, evidencia, salud e índice de capacidades.
Cada componente se guarda como JSON canónico inmutable bajo su hash de contenido.
El commit escribe y sincroniza primero todos los objetos y sólo después publica
el manifest mediante un único rename atómico y `fsync` del directorio.

Los lectores abren una sola vez el manifest activo, verifican cada objeto contra
el hash fijado y ensamblan esa generación. Un objeto huérfano escrito por un
commit fallido es inocuo; el manifest anterior continúa siendo la única fuente
de verdad. Cualquier snapshot de routing es una caché derivada identificada por
el hash del manifest, nunca una entrada manual.

## Compatibilidad y migración

- Los recibos de canario anteriores sin `demonstrated_capabilities` cargan como conjunto
  vacío.
- La política v1 no se carga como v2 de forma implícita. `migrate_v1` requiere que el
  llamador entregue los nuevos valores globales y produce un documento v2 revisable.
- Un documento de evidencia anterior no se convierte por omisión: necesita una
  migración explícita o permanece sólo legible.
- Evidencia, propuestas y eventos de override son append-only. Activar crea una nueva
  generación; no reescribe observaciones, propuestas ni eventos anteriores.

## Hashes

Se serializa JSON canónico (claves de mapas ordenadas, sin espacios) y se calcula SHA-256.
El identificador visible es `sha256:<hex minúsculo>`. El hash de evidencia sella
`RouteRef`, revisión, perfil completo, contribuciones, exclusiones determinantes e
historial de overrides, todos en orden estable. Los recibos conservan tanto el hash
de evidencia activa como el de política.

## Errores mínimos

- `invalid_schema_version`
- `invalid_route_ref`
- `invalid_profile_weights`
- `incompatible_observations`
- `no_usable_evidence`
- `unverified_quality`
- `no_eligible_route`
- `proposal_not_confirmed`
- `proposal_hash_mismatch`

Cada descarte y error público incluye `code`, `field`, `message` y `details`
deterministas.

## Evidencia de implementación

Cada task tiene exactamente un registro JSONL identificado por `task_id`. El
registro declara `evidence_mode = "tdd"` cuando conserva un rojo ejecutado antes
de la implementación, o `evidence_mode = "reconstructed_audit"` cuando sólo se
ha podido reconstruir y verificar trabajo histórico. Una auditoría reconstruida
nunca se presenta como TDD retroactivo.

La evidencia no calcula hashes contra SPEC editables. Cada registro apunta a un
único snapshot inmutable bajo `docs/evidence/specs/`; el nombre del fichero es el
SHA-256 de sus bytes y `spec_sha256` debe coincidir tanto con el nombre como con
el contenido. `spec_paths` conserva la procedencia humana de los documentos que
formaron el snapshot, pero editar esos documentos no altera evidencia histórica.

Para `evidence_mode = "tdd"`, `red.exit_code` debe ser distinto de cero,
`green.exit_code` debe ser cero y una mutación dirigida debe figurar como
detectada. Para `reconstructed_audit`, los comandos son comprobaciones posteriores
y el registro debe explicar explícitamente que no existe un rojo reproducible.
