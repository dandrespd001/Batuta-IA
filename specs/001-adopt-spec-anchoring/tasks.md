# Tasks: Adopción spec-anchored

**Input**: Design documents from `specs/001-adopt-spec-anchoring/`

**Prerequisites**: `plan.md`, `spec.md`, `research.md`, `data-model.md`, `contracts/` y la matriz
CHK001–CHK036 de `checklists/acceptance-evidence.md`

**Tests**: TDD y mutaciones son obligatorios por constitución y por `REQ-SDD-007..010`.

**Organization**: Las tareas se agrupan por historia para conservar incrementos demostrables; dentro
de cada historia, las pruebas rojas preceden a la implementación.

## Gate humano previo a T001

Este gate no es una tarea de implementación ni marca T001–T042. Antes de iniciar Phase 1:

1. El revisor examina las 36 filas de `checklists/acceptance-evidence.md`.
2. Con una única aprobación humana, se cambian exclusivamente las 36 marcas de `acceptance.md` de
   `[ ]` a `[x]`, sin modificar su texto.
3. Se comprueban `acceptance.md` 36/36 y `requirements.md` 16/16 y se repite `$speckit-analyze`.
4. Sólo con análisis verde queda habilitado `$speckit-implement` y puede comenzar T001.

Mientras falte esa aprobación, `acceptance.md` permanece 0/36 y las 42 tareas permanecen sin marcar.

## Format: `[ID] [P?] [Story] Description [Requisitos]`

- **[P]**: Puede hacerse en paralelo porque toca ficheros distintos y no depende de otra tarea pendiente.
- **[Story]**: Historia de usuario de `spec.md`.
- Cada descripción incluye la ruta exacta que cambia.
- **[Requisitos]**: cierre de la línea con los `REQ-SDD-*` de `spec.md` que la tarea realiza. Es la
  fuente que `EvidenceRecordV2` lee para vincular `task` con `requirements` sin reconstruir nada a
  posteriori. `[transversal]` marca las tres tareas de andamiaje o cierre —T003, T040 y T042— que no
  realizan una obligación concreta; ninguna otra tarea puede quedar sin requisito.

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Fijar la herramienta de autoría y el alcance aprobado antes de crear contratos.

- [x] T001 Verificar y conservar el scaffolding Spec Kit v1.0.2 en `.specify/` y `.agents/skills/speckit-*/` [REQ-SDD-001]
- [x] T002 Registrar la constitución aprobada v1.0.0 en `.specify/memory/constitution.md` [REQ-SDD-002]
- [x] T003 Crear la estructura vacía de autoridad en `specs/system/`, `specs/schemas/` y `docs/adr/` [transversal]

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Sellar el baseline e identificar el impacto del propio slice antes de modificar gates.

**⚠️ CRITICAL**: Ninguna historia comienza hasta que estos artefactos hacen auditable el cambio.

- [x] T004 Sellar sobre `7de68af2c9a36ba3dcc65971e4bba83231fb3855` los hashes y tamaños exactos de los seis artefactos V1 y 19 registros en `docs/evidence/v1.sha256` y `docs/evidence/v1-baseline.json`, más el registro de corrida `docs/evidence/baseline.json` en `run_records` [REQ-SDD-007, REQ-SDD-012]
- [x] T020 Publicar `FeatureImpactV1` cerrado en `specs/schemas/feature-impact-v1.schema.json` [REQ-SDD-006]
- [x] T005 Crear la declaración inicial cerrada del cambio, incluida clasificación excluyente y recuperación de migración/rollback, en `specs/001-adopt-spec-anchoring/feature-impact.json` [REQ-SDD-006]

- [x] T006 Registrar las cinco excepciones heredadas con extracción y caracterización en `scripts_ci/modularity_exceptions.json` [REQ-SDD-009]

> **Desorden histórico registrado (T005 ⇄ T020)**: T005 se ejecutó y se marcó antes de que existiera
> su contrato, contra la regla «Esquemas preceden instancias» de §Within Each User Story. T020 se movió
> aquí desde Phase 4 conservando su ID estable. El desorden histórico de 001 se conserva registrado en
> vez de reescribirse.
>
> **Estado de T020**: el artefacto `specs/schemas/feature-impact-v1.schema.json` **ya está escrito**
> —deriva de §FeatureImpactV1 del data-model, reutiliza los patrones de ID canónicos y codifica en
> `if/then` las seis reglas condicionales—, pero **la tarea permanece sin marcar**. El orden interno de
> este paquete es mutaciones → esquema → validador, como demuestra US1 (T007 y T008 preceden a T012 y
> T013), y aquí el esquema se escribió con **T017 todavía pendiente**. Marcarla sería el mismo error
> que este bloque documenta, una vuelta más arriba.
>
> Su verificación provisional —metaesquema draft 2020-12 válido, instancia real aceptada y 16 de 16
> mutaciones de forma rechazadas— se hizo con `jsonschema` **fuera del repositorio, sólo como
> evidencia**: no es dependencia del proyecto ni de ningún gate, y por no haber observado un rojo
> previo su procedencia es `reconstructed_audit`, nunca TDD contemporáneo. Queda registrada en
> `auditorias/batuta-2026-09-01/`. Son mutaciones **de forma del contrato**, distintas de las
> mutaciones **de diagnóstico del validador** que fija §Matriz mínima de mutaciones: T017 debe cubrir
> ambas familias antes de que T020 y T021 puedan cerrarse.

**Checkpoint**: Baseline, alcance, contrato de impacto y deuda quedan explícitos antes de implementar
validadores. Phase 2 **no cierra** mientras T020 siga sin su rojo previo en T017.

---

## Phase 3: User Story 1 - Encontrar la autoridad vigente (Priority: P1) 🎯 MVP

**Goal**: Desde cualquier capacidad se puede recorrer una única autoridad hasta requisitos,
verificaciones, evidencia y roadmap.

**Independent Test**: `validate_spec_anchors.py` acepta el registro inicial y sus mutaciones rechazan
duplicados, huérfanos, estados inválidos y requisitos implementados sin verificación.

### Tests for User Story 1 ⚠️

> Escribir estas pruebas primero y observar que fallan por ausencia del validador/registro.

- [x] T007 [P] [US1] Añadir mutaciones de IDs, estados, campos, rutas, futuro y `implemented` sólo manual de la matriz en `scripts_ci/tests/test_validate_spec_anchors.py` [REQ-SDD-005]
- [x] T008 [P] [US1] Añadir fixtures cerrados válidos e inválidos con diagnósticos/códigos repetibles en `scripts_ci/tests/fixtures/spec_anchors/` [REQ-SDD-005, REQ-SDD-014]

### Implementation for User Story 1

- [x] T009 [P] [US1] Escribir autoridad, lifecycle e IDs en `specs/README.md` y mapa RM-001..RM-008 con enlaces nulos hasta que exista cada paquete en `ROADMAP.md` [REQ-SDD-003, REQ-SDD-004]
- [x] T010 [P] [US1] Crear las specs vivas de producto, manifests y ejecución en `specs/system/product.md`, `specs/system/manifests.md` y `specs/system/execution.md` [REQ-SDD-003]
- [x] T011 [P] [US1] Crear las specs vivas de estado/routing, calidad/research, superficies y rollout en `specs/system/state-policy-routing.md`, `specs/system/quality-research.md`, `specs/system/surfaces.md` y `specs/system/rollout.md` [REQ-SDD-003]
- [x] T012 [US1] Publicar `SpecAnchorRegistryV1` cerrado en `specs/schemas/spec-anchor-registry-v1.schema.json` [REQ-SDD-003]
- [x] T013 [US1] Implementar validación exacta, cobertura y rutas en `scripts_ci/validate_spec_anchors.py` [REQ-SDD-003, REQ-SDD-005, REQ-SDD-014]
- [x] T014 [US1] Crear el inventario completo y biyectivo en `specs/anchors.json`, con roadmap no nulo para `CAP-MANIFESTS`, `CAP-STATE` y `CAP-ROLLOUT` [REQ-SDD-003]
- [x] T015 [P] [US1] Registrar ADR-0001 y la clasificación cerrada por ruta, autoridad, paridad y mutabilidad en `docs/adr/ADR-0001-spec-anchored-governance.md` y `docs/DOCUMENT_CLASSIFICATION.md` [REQ-SDD-011]
- [x] T016 [US1] Añadir avisos de autoridad/supersesión sólo tras paridad total a `docs/ESQUEMA_MANIFIESTO.md`, `docs/CONTRATOS_OPERATIVOS_V2.md`, `docs/ESQUEMA_CALIDAD_ROUTING.md`, `docs/FASE3_EJECUCION.md`, `docs/FASE4_POLITICA.md`, `docs/FASE5_PANEL.md`, `docs/IMPLEMENTACION_ROUTING_V2.md` y `docs/TRAZABILIDAD_ROUTING_V2.md`, conservando la autoridad anterior ante cualquier fila incompleta [REQ-SDD-011]

**Checkpoint**: US1 es navegable y validable sin implementar deriva de diffs ni evidencia V2.

---

## Phase 4: User Story 2 - Cambiar sin deriva silenciosa (Priority: P2)

**Goal**: Los cambios declaran impacto y los gates bloquean omisiones de spec, caracterización,
modularidad o arquitectura.

**Independent Test**: La suite mata cambios funcionales sin spec, refactors sin caracterización,
módulos sobre límite sin excepción, ciclos y dependencias de dominio hacia CLI.

### Tests for User Story 2 ⚠️

- [x] T017 [P] [US2] Añadir en `scripts_ci/tests/test_validate_spec_anchors.py` las dos familias de mutación del impacto: las de **diagnóstico** de §Matriz mínima de mutaciones —clasificación normativa, spec, caracterización, migración recuperable y base Git ausente/inválida— y las de **forma cerrada** de `FeatureImpactV1` —campo desconocido en raíz y anidado, enums fuera de dominio, nulabilidad condicional de `migration` y `rollback`, IDs mal formados y duplicados—. Preceden a T020 y T021 [REQ-SDD-006, REQ-SDD-008]
- [x] T018 [P] [US2] Añadir mutaciones de umbral, duplicado, obsolescencia, ruta y prueba eliminada en `scripts_ci/tests/test_check_modularity.py` [REQ-SDD-009]
- [x] T019 [P] [US2] Añadir fixtures con diagnósticos estables de ciclos, dependencia interna de contrato y dominio hacia CLI en `scripts_ci/tests/test_check_architecture.py` [REQ-SDD-009]

### Implementation for User Story 2

- [x] T021 [US2] Extender correlación de capacidades, requisitos y diff Git en `scripts_ci/validate_spec_anchors.py`: sin base warning/código 0 estructural; base explícita no resoluble código 2 [REQ-SDD-008]
- [x] T022 [P] [US2] Implementar umbrales, warnings y excepciones exactas en `scripts_ci/check_modularity.py` [REQ-SDD-009]
- [x] T023 [P] [US2] Implementar DAG local y fronteras de crates en `scripts_ci/check_architecture.py` [REQ-SDD-009]
- [x] T024 [US2] Integrar validadores y pruebas Python con diagnósticos ordenados/repetibles en `scripts_ci/local_gates.sh` [REQ-SDD-010]
- [x] T025 [US2] Sustituir la lista divergente de CI por checkout con historial completo, base obligatoria y una única llamada a `scripts_ci/local_gates.sh` en `.github/workflows/ci.yml` [REQ-SDD-008, REQ-SDD-010]

**Checkpoint**: US2 bloquea deriva y arquitectura con mutaciones deterministas y sin red.

---

## Phase 5: User Story 3 - Auditar sin reescribir la historia (Priority: P3)

**Goal**: V1 se conserva byte a byte y V2 añade requisitos/snapshot sin mezclar formatos.

**Independent Test**: El validador comprueba hashes V1 y 19 registros, acepta un V2 sellado y rechaza
snapshot alterado, task inexistente, requisito desconocido y campos extra.

### Tests for User Story 3 ⚠️

- [ ] T026 [P] [US3] Ampliar preservación exacta de las siete rutas selladas —los seis artefactos V1 en `artifacts` más el registro de corrida en `run_records`— y de los 19 registros V1, con todas las mutaciones V2 de la matriz, en `scripts_ci/tests/test_validate_tdd_evidence.py` [REQ-SDD-007]
- [ ] T027 [P] [US3] Crear fixtures V2 inválidos por clase en `scripts_ci/tests/fixtures/tdd_evidence/` [REQ-SDD-007]

### Implementation for User Story 3

- [ ] T028 [P] [US3] Publicar `EvidenceRecordV2` cerrado en `specs/schemas/evidence-record-v2.schema.json` [REQ-SDD-007]
- [ ] T029 [US3] Añadir despacho legado/V2, `tdd` frente a `reconstructed_audit`, anchors, task y snapshot a `scripts_ci/validate_tdd_evidence.py` [REQ-SDD-007]
- [ ] T030 [US3] Crear el snapshot direccionado por contenido y el primer registro V2 en `docs/evidence/specs/` y `docs/evidence/tdd-v2.jsonl` [REQ-SDD-007]
- [ ] T031 [US3] Documentar procedencia `reconstructed_audit` y preservación en `specs/system/product.md` y `docs/DOCUMENT_CLASSIFICATION.md` [REQ-SDD-011, REQ-SDD-012]

**Checkpoint**: US3 demuestra continuidad histórica sin modificar ninguno de los seis artefactos V1.

---

## Phase 6: User Story 4 - Repetir el flujo de autoría (Priority: P4)

**Goal**: El siguiente cambio puede usar la versión fijada, instrucciones breves y la misma aceptación
offline local/CI.

**Independent Test**: Un checker offline valida hashes administrados; quickstart y gate completo pasan
desde un checkout sin credenciales, y el estado oficial de integración reporta `ok`.

### Tests for User Story 4 ⚠️

- [ ] T032 [P] [US4] Añadir mutaciones de versión, integración, manifest, ruta, ausencia y hash administrado con diagnósticos exactos en `scripts_ci/tests/test_check_speckit_integration.py` [REQ-SDD-001]

### Implementation for User Story 4

- [ ] T033 [US4] Implementar verificación offline de versión `1.0.2`, manifests, rutas y hashes sin ejecutar Spec Kit en `scripts_ci/check_speckit_integration.py` [REQ-SDD-001]
- [ ] T034 [P] [US4] Escribir instrucciones breves para agentes en `AGENTS.md` [REQ-SDD-013]
- [ ] T035 [P] [US4] Escribir el flujo humano, condiciones de parada, plantilla de impacto/recuperación y autorización externa completa en `CONTRIBUTING.md` [REQ-SDD-013]
- [ ] T036 [US4] Actualizar navegación, estado y gate único en `README.md` [REQ-SDD-013]
- [ ] T037 [US4] Añadir el checker offline a `scripts_ci/local_gates.sh` y contrastar fuera de CI la fuente oficial `specify integration status --json` con v1.0.2 [REQ-SDD-001, REQ-SDD-010]

**Checkpoint**: US4 deja el proceso repetible sin dependencia global ni acceso de proveedores.

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Cerrar trazabilidad, evidencia y aceptación del slice completo.

- [ ] T038 Ejecutar todos los escenarios de `specs/001-adopt-spec-anchoring/quickstart.md`, incluidos tres recorridos ≤5 min y cinco validadores aislados <5 s, y corregir cualquier divergencia documental [REQ-SDD-014]
- [ ] T039 Ejecutar `git diff --check` y `scripts_ci/local_gates.sh`, registrando red/green/mutación reales en `docs/evidence/tdd-v2.jsonl` [REQ-SDD-014]
- [ ] T040 Actualizar las 36 filas de `specs/001-adopt-spec-anchoring/checklists/acceptance-evidence.md` con resultados finales y revalidar `requirements.md` 16/16 y `acceptance.md` 36/36 sin cambiar texto ni ownership [transversal]
- [ ] T041 Actualizar RM-001 y anchors a completado sólo después de gates verdes en `ROADMAP.md` y `specs/anchors.json` [REQ-SDD-004]
- [ ] T042 Ejecutar el cierre `$speckit-analyze` y `$speckit-converge` hasta cero contradicciones y cero tareas nuevas, sin reabrir ni reaprobar el gate humano previo [transversal]

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: Sin dependencias; ya existe el scaffolding generado que debe auditarse.
- **Foundational (Phase 2)**: Depende de Setup y bloquea todas las historias.
- **US1 (Phase 3)**: Depende de Foundational y crea IDs/autoridad consumidos por US2 y US3.
- **US2 (Phase 4)**: Depende de US1 para resolver capacidades e impactos.
- **US3 (Phase 5)**: Depende de US1 para validar requisitos, pero no de los gates de US2.
- **US4 (Phase 6)**: Puede comenzar tras US1; su integración final depende de US2 y US3.
- **Polish (Phase 7)**: Depende de las cuatro historias.

### User Story Dependencies

- **US1 (P1)**: MVP; no depende de otra historia.
- **US2 (P2)**: Usa el registro y specs de US1.
- **US3 (P3)**: Usa IDs de US1; preservación V1 es independiente de US2.
- **US4 (P4)**: Documentación/checker son independientes; gate final reúne US1–US3.

### Within Each User Story

- Pruebas y mutaciones se escriben y fallan antes de implementación.
- Esquemas preceden instancias; instancias preceden validación end-to-end.
- El gate único se modifica sólo cuando cada validador aislado está verde.
- Evidencia V2 se sella después de observar comandos reales.

### Parallel Opportunities

- T004, T006 y T020 tocan artefactos separados después de T001–T003; T005 depende de T020.
- T007/T008, T009/T010/T011 y T015 pueden avanzar en paralelo dentro de US1.
- T017–T019 son pruebas independientes; T022 y T023 implementan fronteras distintas.
- T026/T027 y T028 pueden prepararse en paralelo sin tocar V1.
- T032, T034 y T035 no comparten ficheros.

---

## Parallel Example: User Story 2

```text
Task T017: mutaciones de anchors/impacto en scripts_ci/tests/test_validate_spec_anchors.py
Task T018: mutaciones modulares en scripts_ci/tests/test_check_modularity.py
Task T019: mutaciones de arquitectura en scripts_ci/tests/test_check_architecture.py
```

## Parallel Example: User Story 3

```text
Task T026: mutaciones V1/V2 en scripts_ci/tests/test_validate_tdd_evidence.py
Task T028: contrato cerrado en specs/schemas/evidence-record-v2.schema.json
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Completar Setup y Foundational.
2. Escribir y observar rojas T007–T008.
3. Crear specs, roadmap, registro y validador T009–T016.
4. Detenerse y demostrar navegación/validación de US1 antes de añadir gates de deriva.

### Incremental Delivery

1. US1 entrega autoridad navegable.
2. US2 convierte autoridad en bloqueo automático.
3. US3 añade continuidad auditada V1/V2.
4. US4 hace repetible la autoría y unifica aceptación.
5. Polish sólo declara RM-001 completado con toda la evidencia verde.

### Execution Constraint

Las marcas `[P]` documentan independencia de ficheros, no autorizan agentes secundarios ni efectos
externos. Este slice se implementa sin red, proveedores reales o cambios en `crates/`.

## Notes

- Cada tarea usa un ID estable `TNNN` que `EvidenceRecordV2` puede referenciar.
- No se marca una tarea por intención; sólo después de observar su artefacto y verificación.
- Si aparece un cambio funcional, se detiene la implementación y se repiten spec, plan y analyze.
- Los seis artefactos V1 no se editan bajo ninguna tarea.
