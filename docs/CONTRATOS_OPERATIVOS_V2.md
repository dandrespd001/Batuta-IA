# Contratos operativos del núcleo crítico v2

> **Archivo histórico desde 2026-08-31.** Las autoridades vigentes son
> [`specs/system/execution.md`](../specs/system/execution.md) y
> [`specs/system/state-policy-routing.md`](../specs/system/state-policy-routing.md). La paridad por
> sección está demostrada en [`DOCUMENT_CLASSIFICATION.md`](DOCUMENT_CLASSIFICATION.md); el contenido
> histórico siguiente permanece íntegro e inmutable.

## ExecutionProfileV1

El perfil operativo es un JSON cerrado y sellado. Contiene exclusivamente
`schema_version`, un `workdir` absoluto y canónico, límites positivos e
independientes para stdout y stderr, una gracia positiva de terminación y su
`profile_hash`. El directorio debe existir, ser un directorio y no puede ser la
raíz del sistema. Programa, argv, entorno, secretos, directorio de manifests y
rutas de los almacenes no pertenecen al perfil.

`ExecutionProfileProposalV1` conserva el perfil propuesto, el hash de la base,
un diff revisable y su propio sello. Importar sólo crea staging. Aplicar exige
que coincidan el ID escrito por el operador, `expected_hash`, la base activa y
el sello de la propuesta; la publicación se serializa mediante lease y rename
atómico. El primer perfil activo también pasa por una propuesta confirmada.

## Política mínima de K4

La política de ejecución declara, sin valores implícitos, `max_attempts`,
`max_retry_after_ms` y `max_handoffs`. `max_attempts` y
`max_retry_after_ms` son positivos; `max_handoffs = 0` deshabilita el relevo.
Estos campos viajan en el componente de política de cada generación y quedan
sellados por el manifest usado en cada selección.

## ExecutionGrantV1

Un grant es un documento JSON cerrado e inmutable. Contiene versión 1, ID
seguro, emisión y caducidad Unix UTC, hash del manifest base, rutas exactas,
acciones, operaciones y límites máximos positivos de solicitudes, tokens de
entrada, tokens de salida y milisegundos de pared. El hash canónico excluye sólo
el propio campo `grant_hash`.

`grant create` recibe `ExecutionGrantDraftV1`, que contiene los mismos campos
salvo `grant_hash`. La CLI exige que `manifest_hash` sea el manifest activo y
que todas las rutas ya pertenezcan a su catálogo; después valida y calcula el
sello. Un cliente nunca puede aportar su propio sello ni preautorizar una ruta
que sólo aparezca en una generación futura.

Una generación posterior nunca amplía el grant. Antes de reservar se intersectan
las rutas enumeradas con las todavía elegibles. Una ruta nueva, otra acción u
otra operación exige un grant distinto. Las revocaciones son append-only y
prohíben reservas nuevas; no borran historia ni resultados ya confirmados.

## Ledger y reserva

Cada invocación reserva previamente su máximo de las cuatro dimensiones. El
ledger se actualiza mediante read-modify-write bajo un lease interproceso y se
publica atómicamente. Una confirmación sustituye la reserva por el consumo real y
libera el remanente sólo si el resultado es conocido. Si el resultado es ambiguo,
la reserva completa continúa consumida. Ninguna suma puede superar los límites
del grant.

## RunRequestV2, journal y recuperación

`RunRequestV2` contiene versión 2, ID, objetivo, `TaskSpec`, petición de routing y
`grant_id`; rechaza campos desconocidos. La petición interna de una única llamada
es `InvocationRequestV2 {run_id, route, objective, task, max_output_bytes,
timeout_ms}` y tampoco admite campos adicionales. Nunca contiene programa,
argumentos, entorno, credenciales ni ubicaciones de estado.

`RunStatusV2` expone intentos, reserva total, próxima acción durable,
`next_action_at`, manifest de cada selección, referencia sellada al recibo,
consumo, ruta, journal, checkpoint y `outcome_unknown`. Antes de seleccionar cada
intento se abre una generación vigente y se persiste su hash. Antes de cada
llamada se sincronizan, en orden, `planned`, `reserved` e
`invocation_started`.

El coordinador sincroniza, en este orden, `planned`, `reserved` e
`invocation_started` antes de la única llamada externa. Después sincroniza un
resultado conocido o `outcome_unknown`. Encontrar `invocation_started` sin
resultado tras reinicio produce `outcome_unknown` y nunca reenvía automáticamente.
Un lease por run impide dos rutas activas entre procesos.

Un `Err` del ejecutor después de `invocation_started` siempre termina en
`outcome_unknown`: conserva la reserva máxima, no actualiza un resultado como
conocido y prohíbe retry y fallback. `resume` antes de `next_action_at` devuelve
`probe_not_due` sin invocar. Una reanudación vencida parte del checkpoint
persistido y nunca del historial de conversación.

## HarnessExecutor

`HarnessExecutor::invoke` representa exactamente una llamada. La petición fija
ruta, objetivo, tarea, topes de salida y tiempo. El adaptador abre manifests
únicamente desde la ubicación fijada por `Layout`, resuelve exactamente
harness/proveedor/modelo/revisión, comprueba pin y hash del ejecutable y deriva
programa, argv, materialización y entorno allowlisted sólo del manifest. El
resultado normaliza salida acotada,
tokens, latencia, procedencia y una taxonomía cerrada: rate limit con/sin plazo,
cuota, autenticación, saldo, transitorio, timeout y permanente. El adaptador no
consulta cuentas, saldo, suscripción ni credenciales y no hace operaciones
auxiliares.

Un fallo sólo se clasifica cuando la única llamada aporta el hecho observado.
Una respuesta desconocida o mal formada es `permanent`; nunca habilita retry por
heurística.

## Salud, retry y relevo

La salud conserva exactamente las veinte observaciones más recientes de cada
ruta, incluidos éxitos, fallos conocidos y resultados ambiguos. Un ambiguo se
cuenta conservadoramente como no exitoso. La tasa es éxitos conocidos dividido
por el número de observaciones disponibles (hasta veinte). El p95 usa rango más
próximo sobre latencias ordenadas: `ceil(0.95 * n) - 1`.

Cada observación se publica reemplazando sólo el componente de salud mediante
`StateStore::commit_if_base`. Ante conflicto CAS se recarga la generación, se
reaplica la observación y se reintenta, de modo que una escritura concurrente no
se pierde.

Sólo un rate limit con `Retry-After` puede reintentar la misma ruta. La espera
debe caber simultáneamente en `max_retry_after_ms`, caducidad del grant, deadline
del run, intentos restantes y presupuesto de espera más otro intento. Espera e
intento se reservan atómicamente antes de dormir. Si no procede retry, el relevo
sólo considera rutas no intentadas que siguen elegibles y que el grant enumera.
El `HandoffCheckpoint` lleva objetivo, fallo, hechos, próximo paso y presupuesto
restante; nunca incorpora el historial.

## RunReceiptV2

El recibo final es un JSON cerrado, sellado y append-only. Incluye petición,
grant completo y hash, candidatos evaluados como `{route, action,
candidate_hash}`, descartes, decisiones, reservas, consumos, transiciones,
resultados, checkpoints y estado terminal. El sello excluye únicamente
`receipt_hash`. `RunStatusV2` referencia el ID y hash del recibo; reiniciar no
reescribe sus bytes.

## Superficies K4

Las órdenes públicas son `grant create/status/revoke`, `run`, `run status`,
`run resume` y `executor profile import/status/apply`. Toda respuesta exitosa usa
`ApiResponseV2`. Todo error usa un `ApiErrorV2` cerrado con
`schema_version`, `code`, `field`, `message` y `details`; no existe una forma de
error desnuda alternativa.

## Estado, ensamblado y decisión

`StateManifestV2` es la única raíz de confianza. Sus cinco objetos son documentos
tipados y cerrados; el servicio abre una generación una vez y ensambla exactamente
un candidato por ruta y acción. Toda ausencia de política, salud, evidencia o
capacidad produce un descarte estructurado, nunca un valor por defecto. Los
escritores compiten bajo lease y `commit_if_base` rechaza una base obsoleta.

`RouteDecisionV2` sella manifest, catálogo, política, evidencia, salud,
capacidades y los hashes ordenados de los recibos utilizados. Cada motivo público
se serializa como `{code, field, message, details}`. CLI, MCP y TUI consumen el
mismo `ApiResponseV2`; los errores usan
`{schema_version, code, field, message, details}`.

## Sidecar DSH

El protocolo JSONL cerrado admite una petición `catalog_snapshot` y exactamente
una respuesta correlacionada. El proceso sólo usa `listProviders`, `listModels` y
`resolveModelInfo`; `stream` está fuera de la superficie. La respuesta conserva
identidad, modalidades, contexto y esfuerzos, fija los cuatro costes como
desconocidos y no admite claves, saldo, cuota, suscripción, endpoints privados ni
variables sensibles. El cliente Rust limpia el entorno, aplica allowlist,
timeout, cierre del grupo de procesos y límites separados de stdout/stderr.

## ResearchProposalV2

La propuesta contiene investigador, grant, manifest y evidencia base,
observaciones y fuentes primarias completas. Se ordena y sella canónicamente. Una
observación exige URL, publicación, consulta, benchmark y versión, escenario,
configuración, ruta/revisión, métrica y tipo de fuente coincidentes. La ruta
investigadora no puede certificarse a sí misma. Aplicar exige que no hayan
cambiado manifest, evidencia ni contenido.

## CapabilityCanaryReceiptV2

`ToolEventV2` registra herramienta, éxito, resultado acotado, digest, artefacto y
fuente. Los escenarios de lectura, escritura, herramientas y web requieren un
evento exitoso y efectos verificables fuera de la prosa: nonce exacto, único
artefacto exacto sin escrituras laterales, digest de resultado o URL/estado/digest
observados. El recibo sella ruta/revisión, escenario, manifest, grant, límites,
efectos y vencimiento; sólo es positivo mientras conserva integridad y vigencia.
