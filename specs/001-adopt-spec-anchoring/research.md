# Research: adopción spec-anchored

## Decisión 1 — Persistencia híbrida

**Decision**: Mantener `specs/system/` como contratos vivos y conservar cada `specs/NNN-*` integrado
como registro histórico inmutable.

**Rationale**: Batuta necesita una respuesta única sobre el comportamiento vigente sin perder la
cadena de decisiones y evidencia de cada cambio. El modelo híbrido permite ambas propiedades y está
contemplado por los [modelos de persistencia de Spec Kit](https://github.github.com/spec-kit/concepts/spec-persistence.html).

**Alternatives considered**: Regenerar paquetes antiguos destruiría auditabilidad; usar sólo paquetes
históricos obligaría a recomponer el estado actual a mano; una única spec mutable perdería el contexto
de cada integración.

## Decisión 2 — Adopción brownfield y versión fijada

**Decision**: Sellar primero K4 en el commit
`7de68af2c9a36ba3dcc65971e4bba83231fb3855`, inicializar en la rama de adopción y fijar Spec Kit
`v1.0.2` con integración `codex`, skills bajo `.agents/skills` y scripts `sh`. La salida de
`specify integration status --json` es la fuente oficial de estado; un checker separado valida
offline versión, manifests, rutas y hashes sin ejecutar Spec Kit.

**Rationale**: El baseline hace revisable el scaffolding y permite distinguir reconstrucción histórica
de trabajo nuevo. La [guía brownfield](https://github.github.com/spec-kit/guides/existing-projects.html)
recomienda exactamente esa separación; la integración Codex oficial usa `.agents/skills`.

**Alternatives considered**: Una versión flotante no es reproducible; una instalación global no deja
la versión declarada por el proyecto; vendorear la herramienta completa añade mantenimiento sin valor
para el binario.

## Decisión 3 — Specs vivas por capacidad pública

**Decision**: Crear siete documentos vivos —producto, manifests, ejecución, estado/política/routing,
calidad/research, superficies y rollout— y registrar capacidades más granulares cuando necesiten estado
o roadmap independientes.

**Rationale**: Es la granularidad mínima que permite describir comportamientos públicos, documentos
persistidos, estados y efectos sin especificar funciones privadas. Evita tanto una mega-spec como una
spec por módulo interno.

**Alternatives considered**: Una spec por crate acoplaría la autoridad a la estructura actual; una spec
por requisito fragmentaría la navegación; conservar `FASE*` como autoridad mantendría solapamientos.

## Decisión 4 — Roadmap como spec of specs

**Decision**: `ROADMAP.md` usa IDs `RM-001` a `RM-008`, dependencias, estado, aceptación y enlace al
paquete sólo cuando éste existe. Cada paquete enlaza de vuelta a su entrada.

**Rationale**: Los IDs sobreviven a renombres y permiten que capacidades parciales o externas siempre
tengan trabajo futuro verificable. Sigue el patrón oficial de
[spec of specs](https://github.github.com/spec-kit/concepts/spec-of-specs.html).

**Alternatives considered**: Enlaces a directorios futuros serían referencias rotas; tickets externos
como única fuente no estarían disponibles offline; numerar sólo por orden temporal perdería identidad.

## Decisión 5 — Contratos JSON cerrados y validadores sin dependencias

**Decision**: Publicar `SpecAnchorRegistryV1`, `FeatureImpactV1` y `EvidenceRecordV2` como JSON Schema
2020-12 con `additionalProperties: false`, y hacer que validadores Python de biblioteca estándar
apliquen además invariantes entre documentos.

**Rationale**: JSON Schema documenta el formato para otras herramientas, mientras la validación manual
de claves exactas e invariantes mantiene CI offline y evita añadir un gestor o dependencia al gate.

**Alternatives considered**: Depender de `jsonschema` requeriría instalación externa; sólo JSON Schema
no comprueba existencia de rutas ni cobertura; TOML/YAML añadirían otro parser o reglas ambiguas.

## Decisión 6 — Deriva comparada contra el baseline K4

**Decision**: El registro fija `7de68af2c9a36ba3dcc65971e4bba83231fb3855` como baseline de adopción.
El validador siempre bloquea inconsistencias estructurales y, al recibir una base Git, relaciona rutas
cambiadas con capacidades y paquetes de impacto. Si no se proporciona base, emite el warning estable
`GIT_DIFF_OMITTED`, omite sólo el diff y devuelve `0` si la estructura es válida. Si se proporciona
explícitamente una referencia inválida o ausente en un clon superficial, devuelve `2` y no degrada el
resultado. CI usa `fetch-depth: 0`, requiere base resoluble y la pasa a la misma entrada local.

**Rationale**: La comprobación estructural es determinista en cualquier checkout. La relación con un
diff exige una referencia explícita para no atribuir cambios al commit equivocado. CI proporcionará su
SHA base a la misma entrada local.

**Alternatives considered**: Usar siempre `HEAD^` rompe commits acumulados; depender de `origin/main`
falla offline o con clones sin remoto; ignorar el diff no detecta cambios funcionales sin spec.

## Decisión 7 — Preservación V1 y log V2 separado

**Decision**: No modificar `tdd.jsonl`, `tdd.schema.json` ni los cuatro snapshots V1. Crear un manifest
con SHA-256, bytes y 19 registros esperados; añadir evidencia V2 en un fichero y esquema separados.
La frontera es el commit completo `7de68af2c9a36ba3dcc65971e4bba83231fb3855` y los valores medidos son:

| Artefacto V1 | Bytes | SHA-256 |
|---|---:|---|
| `docs/evidence/tdd.jsonl` | 19311 | `2d8aafbd4f2f32361f9254773de2db08d94d4631c34db447dec7877bc7a29da6` |
| `docs/evidence/tdd.schema.json` | 1837 | `f6032c7a0b36a910803e270e5b60d7eff80ea1699d21a9db326cd31898bd4a89` |
| `docs/evidence/specs/59ddd6234aee1a95fc7db4ecfaeee0ced3befe140190f305f21e00b0f42139f7.md` | 4528 | `59ddd6234aee1a95fc7db4ecfaeee0ced3befe140190f305f21e00b0f42139f7` |
| `docs/evidence/specs/8d1e228e24c449136102608028b2b37403c4624529712d4e00ceba2979999042.md` | 943 | `8d1e228e24c449136102608028b2b37403c4624529712d4e00ceba2979999042` |
| `docs/evidence/specs/9274306a9ad4a83ad9e061e4617d7e547e62f841ebd8c5373a554b17e812a70a.md` | 9417 | `9274306a9ad4a83ad9e061e4617d7e547e62f841ebd8c5373a554b17e812a70a` |
| `docs/evidence/specs/b4ef1a975e590d84f6b29b5787139f1aea3cd7c2d82190c774e6e567c9d42872.md` | 818 | `b4ef1a975e590d84f6b29b5787139f1aea3cd7c2d82190c774e6e567c9d42872` |

`tdd.jsonl` contiene exactamente 19 líneas/registros. El conjunto V1 es exactamente esta tabla: no
incluye `baseline.json`, manifests de preservación nuevos ni futuros artefactos V2.

**Rationale**: Permite demostrar preservación exacta y leer explícitamente el legado, sin forzar una
migración que cambiaría bytes históricos. El validador despacha por versión/fichero y ancla V2 a IDs de
requisito existentes.

**Alternatives considered**: Reescribir V1 a V2 violaría el plan; mezclar dos formas en el mismo JSONL
debilitaría el contrato; confiar sólo en Git no ofrece un gate accionable ante cambios accidentales.

## Decisión 8 — Excepciones modulares cerradas y temporales

**Decision**: El gate advierte desde 400 líneas y falla desde 500 en producción o 700 en tests. Los
módulos heredados sobre el límite requieren una entrada exacta con justificación, extracción y pruebas
de caracterización; entradas obsoletas o incompletas también fallan.

**Rationale**: Los umbrales son señales, no metas. Un registro cerrado hace visible la deuda sin forzar
un refactor riesgoso dentro del slice de gobernanza.

**Alternatives considered**: Un allow por comentario es difícil de auditar; fallar todo el baseline
impediría adoptar el gate; sólo advertir no bloquea crecimiento no justificado.

## Decisión 9 — Una sola entrada de aceptación

**Decision**: `scripts_ci/local_gates.sh` ejecuta todos los validadores, pruebas Python, sidecar,
formato, `no_std`, Clippy con todos los targets/features y tests con todas las features. CI sólo llama
esa entrada y pasa la referencia base obligatoria. Además, en `ubuntu-latest` se mide cada uno de los
cinco validadores Python como proceso independiente con timeout de 5 segundos. Esta medición excluye
Rust, Node, la suite `unittest` agregada y `local_gates.sh`.

**Rationale**: El baseline mostraba divergencia: CI omitía evidencia y `--all-features`. Centralizar la
lista elimina dos definiciones que podían separarse.

**Alternatives considered**: Repetir pasos en YAML conserva la deriva; generar YAML desde shell añade
complejidad; delegar sólo en CI impide reproducir aceptación localmente.

## Decisión 10 — Clasificación antes de supersesión

**Decision**: Inventariar cada documento como `normative`, `evidence`, `decision`, `guide` o `archive`.
Los documentos solapados permanecen intactos hasta que una matriz demuestre paridad total; sólo
entonces se reducen a un aviso de supersesión con enlaces, nunca se borran o alteran snapshots,
medidas o evidencia. Una fila sin destino o prueba conserva la autoridad anterior y permite reintento
idempotente de la publicación.

La asignación cerrada es: manifiestos → `specs/system/manifests.md`; contratos operativos y Fase 3 →
`execution.md` y `state-policy-routing.md`; calidad y Fase 4 → `quality-research.md` y
`state-policy-routing.md`; Fase 5 → `surfaces.md`; implementación/trazabilidad → `ROADMAP.md`, anchors
y evidencia. `README.md` queda como guía, mientras `REGLAS_INGENIERIA_RUST.md` y
`DEUDA_MODULAR_RUST.md` conservan su autoridad específica. La tabla por ruta y clases de ejemplos,
schemas y medidas está en `spec.md` §«Clasificación inicial de documentos existentes».

**Rationale**: Evita dos autoridades y conserva contexto histórico. La clasificación también permite
que el validador rechace más de una autoridad para el mismo requisito.

**Alternatives considered**: Mover todo de una vez arriesga perder contratos; dejar duplicados hace
imposible decidir qué manda; borrar documentos impide auditar decisiones anteriores.

## Decisión 11 — Taxonomías cerradas y recuperación

**Decision**: Derivar `implemented`, `partial`, `external` y `deprecated` de los estados de requisito;
clasificar cada cambio con precedencia `contract > behavior > internal_refactor > docs_only`; y
publicar specs, anchors, roadmap y migraciones desde staging sólo tras validación completa. Todo cambio
normativo es `contract`. Un protocolo manual nunca basta para `implemented`.

**Rationale**: Estados o clases solapadas permitirían elegir la ruta con menos gates. La precedencia y
las invariantes hacen determinista la clasificación. Staging más backup evita que una migración o
supersesión incompleta cambie el estado activo y hace que el reintento sea idempotente.

**Alternatives considered**: Inferir la clase por extensión de fichero falla con specs normativas;
aceptar estado manual permite declarar implementación sin prueba; publicar cada fichero por separado
expone estados intermedios no auditables.

## Matriz mínima de mutaciones

Cada fixture se ejecuta dos veces; debe producir el mismo diagnóstico, orden, bytes y código. Los
códigos `1` representan una invariante rechazada; `2`, una invocación/base no utilizable; el warning de
base ausente conserva `0` cuando no hay otro fallo.

| Clase | Mutación mínima | Diagnóstico estable esperado | Código |
|---|---|---|---:|
| Anchors | Duplicar un ID de capacidad o requisito | `ANCHOR_DUPLICATE_ID` | 1 |
| Anchors | Referenciar spec, ruta, prueba, evidencia o roadmap inexistente | `ANCHOR_PATH_MISSING` | 1 |
| Anchors | Añadir campo o estado desconocido | `SCHEMA_UNKNOWN_FIELD` | 1 |
| Anchors | Dejar `implemented` sólo con `manual_protocol` | `ANCHOR_EXECUTABLE_REQUIRED` | 1 |
| Anchors | Quitar roadmap/protocolo a `partial` o `external` | `ANCHOR_FUTURE_WORK_REQUIRED` | 1 |
| Impacto | Cambiar `crates/` sin paquete de impacto | `IMPACT_REQUIRED` | 1 |
| Impacto | Marcar `docs_only` al tocar spec/schema normativo | `IMPACT_CHANGE_TYPE_CONTRACT` | 1 |
| Impacto | Usar `behavior`/`contract` con `living_specs_updated: false` | `IMPACT_LIVING_SPEC_REQUIRED` | 1 |
| Impacto | Usar `internal_refactor` sin caracterización existente | `IMPACT_CHARACTERIZATION_REQUIRED` | 1 |
| Impacto/Git | Omitir base y mantener estructura válida | `GIT_DIFF_OMITTED` (warning único) | 0 |
| Impacto/Git | Pasar base explícita inválida o ausente en clon superficial | `GIT_BASE_UNRESOLVABLE` | 2 |
| Evidencia V1 | Cambiar un byte de cualquiera de las siete rutas selladas | `EVIDENCE_V1_HASH_MISMATCH` | 1 |
| Evidencia V1 | Añadir o quitar una línea de `tdd.jsonl` | `EVIDENCE_V1_RECORD_COUNT` | 1 |
| Evidencia V2 | Añadir un campo no declarado | `EVIDENCE_V2_UNKNOWN_FIELD` | 1 |
| Evidencia V2 | Referenciar feature, task o requisito inexistente | `EVIDENCE_V2_REFERENCE_UNKNOWN` | 1 |
| Evidencia V2 | Alterar bytes, nombre o hash del snapshot | `EVIDENCE_V2_SNAPSHOT_HASH` | 1 |
| Evidencia V2 | Registrar rojo TDD con código cero | `EVIDENCE_V2_RED_NOT_FAILING` | 1 |
| Modularidad | Superar 500/700 líneas sin excepción | `MODULARITY_LIMIT_EXCEEDED` | 1 |
| Modularidad | Duplicar una excepción o dejarla bajo umbral | `MODULARITY_EXCEPTION_DUPLICATE` / `MODULARITY_EXCEPTION_STALE` | 1 |
| Modularidad | Borrar ruta o prueba de caracterización de una excepción | `MODULARITY_CHARACTERIZATION_MISSING` | 1 |
| Arquitectura | Introducir ciclo entre crates locales | `ARCHITECTURE_CYCLE` | 1 |
| Arquitectura | Hacer que `batuta-contract` dependa de crate interno | `ARCHITECTURE_CONTRACT_DEPENDENCY` | 1 |
| Arquitectura | Añadir dependencia de dominio hacia `batuta-cli` | `ARCHITECTURE_DOMAIN_TO_CLI` | 1 |
| Spec Kit | Alterar `integration.json` a otra versión/integración | `SPECKIT_VERSION_MISMATCH` | 1 |
| Spec Kit | Borrar un fichero administrado | `SPECKIT_MANAGED_FILE_MISSING` | 1 |
| Spec Kit | Alterar un fichero sin actualizar su manifest oficial | `SPECKIT_MANAGED_HASH_MISMATCH` | 1 |
| Spec Kit | Añadir ruta absoluta, `..` o ruta fuera de raíz al manifest | `SPECKIT_MANAGED_PATH_INVALID` | 1 |

## Protocolo mínimo de navegación

El revisor hace tres recorridos independientes, reiniciando el cronómetro en cada fila. Empieza con
`specs/anchors.json` abierto y detiene el reloj al mostrar todos los destinos; T014 debe asignar un
`roadmap_id` no nulo a estas tres capacidades para que el recorrido sea completo.

| Caso | Destinos obligatorios desde el anchor | Límite individual |
|---|---|---:|
| `CAP-MANIFESTS` | `owner_spec` → `specs/system/manifests.md`; `status`; al menos un `REQ-*`; su verificación/evidencia; `roadmap_id` → `ROADMAP.md` | ≤ 5 min |
| `CAP-STATE` | `owner_spec` → `specs/system/state-policy-routing.md`; `status`; al menos un `REQ-*`; su verificación/evidencia; `roadmap_id` → `ROADMAP.md` | ≤ 5 min |
| `CAP-ROLLOUT` | `owner_spec` → `specs/system/rollout.md`; `status`; al menos un `REQ-*`; su verificación/evidencia; `roadmap_id` → `ROADMAP.md` | ≤ 5 min |

Cada caso falla si falta un destino, supera cinco minutos o aparece un segundo documento normativo que
afirme el mismo contrato. El protocolo mide navegación; no sustituye los gates estructurales.
