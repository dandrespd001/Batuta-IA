# Data Model: adopción spec-anchored

## Reglas comunes

Todos los objetos JSON son cerrados: las claves enumeradas son las únicas admitidas y cada schema
usa `additionalProperties: false`, también en objetos anidados. Los IDs y enums se serializan en ASCII;
las rutas son relativas a la raíz, usan `/`, no contienen `..`, apuntan a ficheros existentes y se
resuelven sin seguir enlaces fuera del repositorio. Las listas declaradas como conjuntos están
ordenadas lexicográficamente y no admiten duplicados. `null` es distinto de campo ausente.

Una **verificación** es una prueba o gate ejecutable, o bien un protocolo manual; describe cómo
comprobar. La **evidencia** registra que una verificación ya se ejecutó. Un **snapshot** conserva los
bytes normativos usados por esa evidencia. `reconstructed_audit` es procedencia posterior y nunca
prueba que existiera un ciclo red-green contemporáneo.

## SpecAnchorRegistryV1

Registro raíz que enlaza el estado vivo con evidencia ejecutable.

### Campos raíz

| Campo | Regla |
|---|---|
| `schema_version` | Constante entera `1` |
| `baseline_commit` | Constante `7de68af2c9a36ba3dcc65971e4bba83231fb3855` para el inventario inicial |
| `generated_from` | Ruta exacta `specs/system/product.md` |
| `capabilities` | Lista no vacía, ordenada por ID y sin duplicados |

### CapabilityAnchorV1

| Campo | Regla |
|---|---|
| `id` | `CAP-*`, único y estable |
| `title` | Nombre no vacío en español |
| `owner_spec` | Un fichero existente bajo `specs/system/` |
| `code_paths` | Prefijos existentes y no solapados dentro de la misma capacidad |
| `status` | `implemented`, `partial`, `external` o `deprecated` |
| `requirements` | Uno o más anchors de requisito, sin IDs repetidos globalmente |
| `evidence` | Lista ordenada no vacía de rutas a registros de evidencia o auditoría; no incluye protocolos |
| `roadmap_id` | ID `RM-NNN`; obligatorio en `partial`, `external` y `deprecated`; nulo sólo en `implemented` sin evolución abierta |
| `protocol` | Ruta de aceptación; obligatoria para `partial` y `external`, nula en otro caso |

Los estados se derivan, no se eligen libremente:

| Estado de capacidad | Invariante excluyente |
|---|---|
| `implemented` | Todos los requisitos activos son `implemented` y cada uno tiene `test` o `gate` |
| `partial` | Hay al menos una parte observable local y al menos un requisito activo `partial` o `external`; incluye cualquier mezcla de implementado y externo |
| `external` | Todos los requisitos activos son `external` y dependen de autorización o sistema externo |
| `deprecated` | Todos los requisitos son `deprecated`; conserva evidencia y roadmap de migración/retirada |

`owner_spec` contiene exactamente una declaración de la capacidad y de cada uno de sus requisitos.
La unión de los `id` de las siete specs vivas debe ser igual a la de `capabilities`; la misma biyección
se aplica a los IDs de requisito. Un roadmap de capacidad parcial/externa debe enumerar en aceptación
cada requisito no implementado y el protocolo debe cubrirlos explícitamente.

### RequirementAnchorV1

| Campo | Regla |
|---|---|
| `id` | `REQ-<CAPACIDAD>-NNN`, único globalmente y presente en `owner_spec` |
| `statement` | Conducta testable no vacía |
| `status` | `implemented`, `partial`, `external` o `deprecated`, con las mismas definiciones objetivas |
| `verifications` | Lista de verificaciones existentes |

Cada verificación tiene exactamente `kind`, `path` y `selector`:

| Campo | Regla |
|---|---|
| `kind` | `test`, `gate` o `manual_protocol` |
| `path` | Fichero existente; para `test`/`gate` forma parte de un comando offline invocable |
| `selector` | Nombre de caso/función para `test`, argumento o regla para `gate`, y nulo para `manual_protocol` |

`test` y `gate` son ejecutables y deben tener un selector resoluble; `manual_protocol` contiene pasos,
precondiciones, observaciones y resultado esperado, pero no es ejecutable. Todo requisito
`implemented` necesita al menos una entrada `test` o `gate`; uno enlazado únicamente a
`manual_protocol` es inválido con `ANCHOR_EXECUTABLE_REQUIRED`. `partial` y `external` necesitan al
menos un protocolo manual, además de cualquier prueba de la parte ya existente.

## FeatureImpactV1

Declaración de impacto que vive dentro de cada paquete de cambio.

| Campo | Regla |
|---|---|
| `schema_version` | Constante entera `1` |
| `feature_id` | Coincide exactamente con el nombre del directorio `NNN-*` |
| `change_type` | `behavior`, `contract`, `internal_refactor` o `docs_only` |
| `capabilities` | IDs existentes, únicos y ordenados |
| `requirements` | IDs existentes y pertenecientes a las capacidades declaradas |
| `compatibility` | Objeto cerrado `public_contract`, `persisted_data`, `notes` |
| `migration` | Objeto cerrado `required`, `plan`, `backup`, `retry` |
| `rollback` | Objeto cerrado `strategy`, `procedure`, `success_check` |
| `living_specs_updated` | Debe ser `true` para `behavior` y `contract` |
| `characterization` | Lista ordenada de rutas existentes; no vacía para `internal_refactor` |

`compatibility.public_contract` y `compatibility.persisted_data` usan exactamente `compatible`,
`incompatible` o `not_applicable`; `notes` es texto no vacío que justifica ambos valores. `migration`
usa un booleano y tres cadenas o nulos. Si `required` es `true`, `plan`, `backup` y `retry` son cadenas
no vacías y deben demostrar respectivamente pasos verificables, restauración del estado previo y
reintento idempotente; si es `false`, las tres son nulas. Una incompatibilidad en cualquiera de las
dos dimensiones obliga a `required: true`.

`rollback.strategy` usa `revert`, `restore_backup`, `forward_fix` o `not_applicable`. Con las tres
primeras opciones, `procedure` y `success_check` son cadenas no vacías; con `not_applicable`, ambas
son nulas y sólo se admite `docs_only`. Una migración fallida no publica staging: conserva el estado
activo, mantiene el backup verificable y puede repetirse con la misma entrada sin efectos duplicados.

La clasificación usa esta precedencia para ser excluyente:

1. `contract`: cambia cualquier obligación normativa, spec viva, schema, versión, vocabulario, forma
   pública/persistida o regla de compatibilidad, incluso sin código o junto con conducta.
2. `behavior`: cambia un resultado observable dentro de un contrato idéntico.
3. `internal_refactor`: toca producto y la caracterización demuestra igualdad de API, mensajes,
   serialización, hashes, decisiones, efectos y bytes relevantes.
4. `docs_only`: sólo cambia prosa no normativa o navegación.

`docs_only` no puede declarar cambios bajo `crates/`. `internal_refactor` no puede marcar una spec viva
como cambio conductual, pero sí puede actualizar trazabilidad. Ambos requieren
`living_specs_updated: false`; `behavior` y `contract` requieren `true`. Un cambio incompatible exige
migración y rollback no trivial. Tocar una spec viva o schema bajo `docs_only` falla con
`IMPACT_CHANGE_TYPE_CONTRACT`.

## EvidenceRecordV2

Evidencia nueva separada del legado V1.

| Campo | Regla |
|---|---|
| `schema_version` | Constante entera `2` |
| `feature_id` | Paquete existente |
| `task_id` | ID `TNNN` presente en `tasks.md` del paquete |
| `requirement_ids` | Lista no vacía de requisitos existentes en anchors |
| `evidence_mode` | `tdd` o `reconstructed_audit` |
| `spec_snapshot` | Objeto cerrado con `path` direccionado por contenido y `sha256` coincidente |
| `red` | Comando, código distinto de cero y resumen no vacío |
| `green` | Comando, código cero y resumen no vacío |
| `mutation` | Comando, código, resumen y `killed: true` |
| `recorded_at` | Timestamp UTC explícito |

`spec_snapshot` tiene exactamente `path` y `sha256`: `path` coincide con
`docs/evidence/specs/<sha256>.md`, y los tres valores —nombre, campo y hash de bytes— deben ser iguales.
`red` y `green` tienen exactamente `command`, `exit_code` y `summary`; `mutation` añade únicamente
`killed`. Los códigos son enteros no negativos, `red.exit_code != 0`, `green.exit_code == 0` y
`mutation.killed == true`; el código de mutación conserva el valor real del runner. Comandos y
resúmenes son no vacíos y no contienen secretos.

Con `evidence_mode: tdd`, el rojo fue observado antes del verde dentro del paquete. Con
`reconstructed_audit`, el registro se creó después del comportamiento: el resumen rojo debe empezar
por `reconstructed audit:` y no puede satisfacer una obligación de TDD contemporáneo, aunque sí
demostrar el estado actual. El inventario K4 conserva esa procedencia donde corresponda. Un registro
se vuelve inmutable al integrarse; una corrección crea otro registro, nunca edita el anterior.

## RoadmapEntry

| Campo | Regla |
|---|---|
| `id` | `RM-NNN`, permanente |
| `title` | Nombre no vacío |
| `scope` | Resultado acotado, no una lista abierta |
| `dependencies` | Lista de IDs anteriores o condición externa explícita |
| `status` | `done`, `in_progress`, `planned`, `external` o `deprecated` |
| `acceptance` | Criterios observables e IDs exactos de requisitos/capacidades afectados |
| `package` | Ruta o nulo; enlace bidireccional sólo cuando el directorio existe |
| `external_authorization` | Objeto cerrado o nulo con `scope`, `routes`, `limits`, `budget`, `expires_at`, `rollback` |

Las entradas RM-002..RM-008 comienzan con `package: null`. Al crear un paquete, una única propuesta
añade la ruta al roadmap y el `roadmap_id` inverso al impacto/anchors; si cualquiera falta, no se
publica el estado. Un efecto externo exige todas las cadenas/listas de autorización no vacías,
caducidad UTC futura y aprobación humana separada; 001 no crea ni consume esa autorización.

## DocumentClassification

Cada fila tiene exactamente `path`, `class`, `successor_authority`, `parity_verified`,
`parity_evidence` y `mutability`. `class` es `normative`, `evidence`, `decision`, `guide` o `archive`;
`mutability` es `living`, `append_only` o `immutable`. `successor_authority` y `parity_evidence` son
rutas o nulos. La asignación inicial por documento está cerrada en `spec.md` §«Clasificación inicial
de documentos existentes».

`evidence` nunca admite supersesión destructiva. Una fila `normative` sólo pasa a `archive` cuando
`parity_verified` es verdadero, `successor_authority` existe, cada afirmación normativa aparece en la
matriz de paridad y `parity_evidence` demuestra sus verificaciones. Si falta una sola, el documento
anterior conserva contenido y autoridad. Convertirlo antes en enlace falla con
`DOCUMENT_SUPERSESSION_INCOMPLETE`; snapshots, medidas y evidencia nunca se reescriben.

## ModularityExceptionV1

Cada excepción tiene exactamente `path`, `kind`, `justification`, `extraction` y
`characterization_tests`. `kind` es `production` o `test`; la ruta y todas las pruebas deben existir,
las listas no pueden estar vacías y `extraction` nombra una responsabilidad concreta. El gate advierte
desde 400 líneas físicas formateadas, falla desde 500 en producción o 700 en tests, y rechaza
excepciones para ficheros bajo el umbral de fallo, duplicadas, vacías, con clase equivocada, ruta
ausente o pruebas eliminadas. Una excepción se retira en el mismo cambio que deja el fichero bajo el
umbral.

## State Transitions

### Paquete de cambio

```text
draft -> clarified -> planned -> analyzed -> implementing -> integrated/historical
```

No se puede entrar en `implementing` con análisis rojo. `integrated/historical` es terminal; una
evolución crea otro paquete.

### Capacidad viva

```text
external -> partial -> implemented -> deprecated
                 \------> deprecated
```

Una transición hacia `implemented` exige verificaciones ejecutables para todos sus requisitos. Una
deprecación conserva requisitos, evidencia y ruta de migración.

### Publicación y supersesión

```text
draft/staging -> validated -> active
        |            |
        +--failure---+--> previous active + recoverable draft/backup
```

Sólo una publicación validada cambia el puntero activo. Un fallo de schema, paridad, hash, escritura o
CAS conserva el activo anterior. Repetir con la misma base y entrada no duplica evidencia ni cambia
bytes. La supersesión sigue la misma transición y no modifica el documento anterior hasta paridad
total.

### Gate de deriva

```text
report (inventario incompleto) -> enforce (cobertura y baseline verdes)
```

La transición a `enforce` requiere: capacidades registradas, requisitos implementados verificados,
parciales/externos con roadmap, autoridad documental única y todos los gates verdes.
