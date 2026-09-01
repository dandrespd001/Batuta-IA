# Feature Specification: Adopción spec-anchored

**Feature Branch**: `sdd/spec-anchored-adoption`

**Roadmap**: [`RM-001`](../../ROADMAP.md#rm-001)

**Created**: 2026-08-31

**Status**: Draft

**Input**: Adopción brownfield de un modelo SDD híbrido en el que las especificaciones vivas,
las pruebas y el comportamiento evolucionan juntos, mientras cada paquete de cambio y su evidencia
integrada permanecen inmutables.

## Vocabulario normativo y autoridad

Una **autoridad viva** es el único artefacto normativo que describe el contrato vigente de una
capacidad. Está identificado por `owner_spec` bajo `specs/system/`, puede evolucionar únicamente en
un paquete nuevo aprobado y debe cambiar junto con sus pruebas y, cuando corresponda, su código. Un
**paquete histórico** es un directorio `specs/NNN-*` ya integrado: registra qué se aprobó en aquel
cambio y no se modifica para describir el presente. Sus checklists, impacto, evidencia y snapshots
son historial inmutable. Guías, ADR, roadmap, inventarios y evidencia pueden dirigir, explicar o
demostrar, pero no sustituyen por sí solos a la spec viva propietaria.

Los términos de aceptación tampoco son intercambiables:

| Término | Significado verificable | Qué no demuestra |
|---|---|---|
| **Prueba ejecutable** | Comando local determinista que comprueba conducta y devuelve `0` al pasar | Que un efecto externo real haya ocurrido |
| **Gate ejecutable** | Validador offline determinista que comprueba una invariante de repositorio y usa códigos `0/1/2` | Conducta que el gate no ejecuta ni inspecciona |
| **Protocolo manual** | Pasos, precondiciones, observaciones y criterio de aceptación reproducibles por una persona | Implementación; nunca basta como único enlace de un requisito `implemented` |
| **Evidencia** | Registro sellado del comando, resultado y requisito que ya se verificó | Una regla normativa nueva ni una prueba que todavía no existe |
| **Snapshot** | Copia inmutable direccionada por su SHA-256 de los bytes normativos usados al registrar evidencia | La autoridad vigente después de cambios posteriores |
| **`reconstructed_audit`** | Verificación posterior de trabajo histórico cuya cronología red-green no fue observada | TDD contemporáneo ni un rojo anterior a la implementación |

`DEBE` expresa obligación comprobable. Cuando dos documentos difieran, prevalecen en este orden: la
constitución, la spec viva propietaria, el contrato/schema vivo y, por último, guías o historia. Un
documento anterior conserva autoridad hasta que la paridad completa y el enlace sustituto hayan sido
validados; no existe supersesión parcial.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Encontrar la autoridad vigente (Priority: P1)

Una persona que mantiene Batuta puede partir de una capacidad del producto, localizar su contrato
vigente, conocer su estado real y seguir el enlace a su trabajo pendiente sin decidir entre varios
documentos contradictorios.

**Why this priority**: Sin una autoridad única no es posible bloquear la deriva ni revisar cambios
posteriores de manera consistente.

**Independent Test**: Elegir cualquier capacidad registrada y recorrer sus enlaces hasta la spec viva,
los requisitos, las verificaciones y la entrada de roadmap; todos existen y expresan el mismo estado.

**Acceptance Scenarios**:

1. **Given** una capacidad implementada, **When** una persona consulta el registro, **Then** encuentra
   una spec propietaria y al menos una verificación ejecutable por cada requisito implementado.
2. **Given** una capacidad parcial o externa, **When** una persona consulta el registro, **Then**
   encuentra una entrada de roadmap y un protocolo que explica qué falta y cómo se aceptará.
3. **Given** un documento normativo anterior con contenido duplicado, **When** se completa su
   clasificación, **Then** queda una sola autoridad y el documento anterior enlaza a ella sin fingir
   que el historial desapareció.

---

### User Story 2 - Cambiar sin deriva silenciosa (Priority: P2)

Una persona que propone un cambio declara su impacto, actualiza los contratos vivos afectados y recibe
un fallo accionable si omite requisitos, pruebas, compatibilidad, migración o rollback.

**Why this priority**: Es la garantía operativa que convierte la documentación en un contrato y no en
una narración opcional.

**Independent Test**: Ejecutar el gate con mutaciones controladas —ID duplicado, referencia huérfana,
requisito sin verificación y cambio funcional sin impacto— y comprobar que cada una es rechazada con
un diagnóstico estable.

**Acceptance Scenarios**:

1. **Given** un cambio de comportamiento con código pero sin actualización de spec viva, **When** se
   ejecutan los gates, **Then** el cambio queda bloqueado.
2. **Given** un cambio normativo de esquema, vocabulario o spec viva, aunque no cambie código, **When**
   se clasifica su impacto, **Then** se registra como `contract` y se declaran compatibilidad y la
   decisión verificable de migración.
3. **Given** un refactor interno con caracterización e impacto declarado, **When** se ejecutan los
   gates, **Then** no se exige inventar un cambio funcional.
4. **Given** un cambio limitado a prosa no normativa y sin rutas de producto, schemas o specs vivas,
   **When** se valida como `docs_only`, **Then** no se exige fingir un cambio de contrato.
5. **Given** un requisito parcial sin trabajo futuro enlazado, **When** se valida el registro, **Then**
   el error identifica la capacidad y el requisito concretos.

---

### User Story 3 - Auditar sin reescribir la historia (Priority: P3)

Una persona auditora distingue evidencia TDD contemporánea de reconstrucciones históricas y puede
demostrar que los 19 registros V1 y sus snapshots no fueron alterados durante la adopción.

**Why this priority**: Atribuir retroactivamente un ciclo red-green o reescribir evidencia invalidaría
la confianza que el modelo pretende crear.

**Independent Test**: Comparar los bytes y sellos del baseline V1, validar un registro V2 vinculado a
requisitos y confirmar que el inventario K4 usa la procedencia `reconstructed_audit`.

**Acceptance Scenarios**:

1. **Given** un registro histórico V1, **When** se valida después de la adopción, **Then** conserva sus
   bytes y se procesa explícitamente como legado.
2. **Given** evidencia creada para un cambio nuevo, **When** se valida, **Then** nombra feature, task,
   requisitos, snapshot sellado y evidencia red/green/mutación aplicable.
3. **Given** una capacidad reconstruida desde K4, **When** se consulta su procedencia, **Then** nunca se
   presenta como TDD anterior a la prueba.

---

### User Story 4 - Repetir el flujo de autoría (Priority: P4)

Una persona contribuidora puede iniciar el flujo SDD con la versión fijada de la herramienta, seguir
instrucciones breves y ejecutar una única definición offline de aceptación tanto localmente como en CI.

**Why this priority**: La gobernanza sólo perdura si el siguiente cambio puede repetirla sin depender
de conocimiento oral o de una versión flotante.

**Independent Test**: Desde un checkout limpio, verificar la integración administrada, seguir el flujo
documentado de un cambio sin efectos externos y obtener el mismo resultado de gates local y en CI.

**Acceptance Scenarios**:

1. **Given** un checkout limpio, **When** se verifica la integración de autoría, **Then** se identifica
   de forma inequívoca la versión fijada y todos sus artefactos administrados están íntegros.
2. **Given** un cambio sin red, **When** se ejecuta la aceptación local, **Then** todas las comprobaciones
   también usadas por CI se ejecutan desde una sola entrada.

### Edge Cases

- Dos capacidades o requisitos intentan reutilizar el mismo ID estable.
- Una ruta de código, prueba, evidencia, spec o roadmap registrada deja de existir.
- Una capacidad `partial` o `external` afirma no tener trabajo pendiente.
- Una capacidad `implemented` carece de una verificación ejecutable para uno de sus requisitos.
- Una capacidad `implemented` enlaza únicamente un protocolo manual.
- Un esquema acepta campos desconocidos o una referencia a una capacidad no registrada.
- Un cambio toca código de producto sin paquete de impacto, o sólo documentación histórica inmutable.
- Un refactor declarado altera mensajes, serialización, hashes, decisiones o bytes caracterizados.
- No se proporciona una base Git: la estructura se valida, el diff se omite con un warning estable y
  el proceso conserva código `0` si no hay otros fallos.
- Se proporciona una base Git explícita inválida o ausente en un clon superficial: la invocación
  termina con código `2`; CI usa historial completo y exige una base resoluble.
- Una migración falla: el estado activo anterior permanece intacto, el backup sigue recuperable y
  repetir el intento con la misma entrada es idempotente.
- La paridad de una supersesión queda incompleta: el documento anterior conserva autoridad y no se
  reduce a enlace hasta que toda afirmación normativa tenga destino y verificación.
- Falla la publicación coordinada de anchors, roadmap o spec viva: ningún estado parcial se vuelve
  activo; el borrador se conserva para diagnóstico y un reintento produce el mismo resultado.
- La herramienta de autoría no está instalada globalmente o intenta usar una versión distinta.
- Un gate intenta acceder a red, credenciales o cuota durante CI.

## Requirements *(mandatory)*

### Alcance

Este slice adopta la gobernanza, inventaría el estado K4 y crea los contratos y gates que harán
verificables los slices posteriores. Incluye la constitución, specs vivas iniciales, roadmap, ADR,
registro de anchors, contratos de impacto/evidencia, clasificación documental e integración de gates.
No incluye los refactors 002–003, cambios funcionales 004–006, campañas reales 007 ni corte V1 008.
Esos siete paquetes todavía no existen: sus entradas de roadmap mantienen `package: null` y sólo
adquieren enlace bidireccional cuando se crea el directorio correspondiente. El slice 001 no modifica
`crates/`, comportamiento público ni ninguno de los seis artefactos V1. Si aparece una necesidad
funcional, se detiene la implementación y se repiten spec, plan y análisis en el paquete adecuado.

### Functional Requirements

- **REQ-SDD-001**: El repositorio DEBE fijar una versión única de la herramienta de autoría y poder
  verificar automáticamente la integridad de su integración sin convertirla en dependencia del
  producto ejecutable.
- **REQ-SDD-002**: La constitución DEBE declarar principios obligatorios, procedimiento de enmienda,
  versión y fechas, y toda modificación normativa DEBE requerir aprobación humana explícita.
- **REQ-SDD-003**: Cada capacidad pública vigente DEBE tener un ID estable, una spec viva propietaria,
  estado cerrado y excluyente, requisitos testables con estado propio, rutas afectadas,
  verificaciones, evidencia y entrada de roadmap cuando corresponda. El conjunto de IDs declarado
  por las specs vivas y el conjunto registrado en anchors DEBEN ser idénticos.
- **REQ-SDD-004**: El roadmap DEBE conservar IDs permanentes, alcance, dependencias, estado, criterios
  de aceptación y enlaces bidireccionales con cada paquete de cambio.
- **REQ-SDD-005**: El registro de anchors DEBE rechazar IDs duplicados, estados desconocidos,
  referencias inexistentes, requisitos implementados sin verificación y estados parciales o externos
  sin roadmap ni protocolo.
- **REQ-SDD-006**: Cada paquete nuevo DEBE incluir una declaración cerrada de impacto que clasifique el
  cambio, enumere capacidades y requisitos afectados y explicite compatibilidad, migración, rollback y
  actualización de specs vivas. Las clases son excluyentes y cualquier cambio normativo de schema,
  vocabulario, obligación o spec viva cuenta como `contract`, aunque no cambie código.
- **REQ-SDD-007**: La evidencia nueva DEBE vincular feature, task, requisitos y snapshot sellado, y
  distinguir evidencia red, green y de mutación; la evidencia V1 existente DEBE seguir siendo legible
  y conservarse byte a byte.
- **REQ-SDD-008**: El gate de deriva DEBE detectar cambios de producto sin paquete de impacto, cambios
  funcionales sin spec viva y refactors sin caracterización, con pruebas de mutación deterministas.
  Sin base Git DEBE completar la validación estructural, emitir `GIT_DIFF_OMITTED` y omitir sólo la
  correlación de diff; una base explícita no resoluble DEBE producir código `2`.
- **REQ-SDD-009**: Los límites modulares y de arquitectura DEBEN validarse offline, admitir sólo
  excepciones registradas con extracción y pruebas concretas y comprobar las dependencias prohibidas.
- **REQ-SDD-010**: Una única entrada de aceptación DEBE ejecutar el mismo conjunto de gates localmente
  y en CI, incluidos formato, contrato sin E/S, anchors, evidencia, modularidad, arquitectura, sidecar,
  análisis estático y pruebas completas. CI sólo puede aportar la base Git como dato a esa misma
  entrada, no mantener una segunda lista de pasos.
- **REQ-SDD-011**: Los documentos actuales DEBEN clasificarse como normativos, evidencia, decisión,
  guía o archivo; los contratos duplicados sólo pueden convertirse en enlaces de supersesión después
  de demostrar paridad total, y ningún snapshot, medida o evidencia histórica puede reescribirse.
- **REQ-SDD-012**: El baseline K4 reconstruido DEBE identificarse como `reconstructed_audit` y el
  modelo documental DEBE explicar que los paquetes integrados son históricos mientras las specs de
  sistema continúan vivas.
- **REQ-SDD-013**: Las instrucciones para agentes y personas contribuidoras DEBEN ser breves, enlazar
  la autoridad normativa y describir cuándo detener código, actualizar spec y repetir planificación y
  análisis.
- **REQ-SDD-014**: Todos los validadores y contratos creados por este slice DEBEN rechazar campos
  desconocidos y producir resultados deterministas sin red ni credenciales. Para fixtures idénticos,
  cada proceso DEBE devolver los mismos bytes en `stdout`/`stderr` y el mismo código `0`, `1` o `2`,
  con diagnósticos ordenados, rutas relativas y sin valores de secretos. Cada validador Python aislado
  DEBE completar en menos de 5 segundos en `ubuntu-latest`.

### Estados cerrados de capacidad y requisito

El estado se calcula sobre requisitos no deprecados y las cuatro definiciones son excluyentes:

| Estado | Condición objetiva |
|---|---|
| `implemented` | La obligación completa existe en el baseline activo y cada requisito tiene al menos una prueba o gate ejecutable existente; no hay requisitos `partial` ni `external` |
| `partial` | Existe una parte observable, pero al menos una obligación no está completa; cada hueco tiene roadmap y protocolo, e incluye cualquier mezcla de requisitos implementados y externos |
| `external` | Toda obligación activa depende de una autorización o sistema fuera del repositorio y ninguna se declara implementada localmente; tiene roadmap y protocolo |
| `deprecated` | Ya no se admite para uso nuevo; todos sus requisitos están deprecados y se conservan compatibilidad, evidencia, migración y roadmap de retirada |

Una transición a `implemented` sólo es válida después de que todos los requisitos activos sean
`implemented`. `external -> partial -> implemented -> deprecated` y `partial -> deprecated` son las
únicas transiciones directas. Reactivar una capacidad deprecada exige un ID o paquete nuevo y revisión
de compatibilidad; no se cambia silenciosamente su estado histórico.

### Clases cerradas de cambio

Se aplica la primera regla que corresponda, en este orden, para obtener una sola clase:

| Clase | Criterio observable |
|---|---|
| `contract` | Cambia una obligación normativa, spec viva, schema, versión, vocabulario, forma persistida/pública o compatibilidad; tiene precedencia aunque también cambie ejecución |
| `behavior` | Cambia un resultado observable para la misma entrada/estado dentro de un contrato que permanece idéntico |
| `internal_refactor` | Toca producto, pero las caracterizaciones demuestran igualdad de API, mensajes, serialización, hashes, decisiones, efectos y bytes relevantes |
| `docs_only` | Sólo cambia prosa no normativa o navegación; no toca `crates/`, specs vivas, schemas, constitución, contratos públicos ni evidencia histórica |

### Publicación, migración y autorización externa

Una actualización multiarchivo se prepara fuera del estado activo y sólo se publica cuando schemas,
anchors, roadmap, paridad y gates están verdes. Ante fallo se conserva íntegro el estado activo, se
mantiene un backup recuperable cuando hay migración y el mismo intento puede repetirse sin duplicar
registros ni cambiar el resultado. Una supersesión incompleta se considera publicación fallida: el
documento anterior sigue siendo normativo.

Aunque 001 no ejecuta efectos externos, cualquier paquete futuro que los solicite necesita una
autorización humana independiente con alcance, rutas, límites operativos, presupuesto, caducidad y
procedimiento de rollback explícitos. La ausencia o expiración de cualquiera de esos datos bloquea el
efecto y no afecta a los gates offline.

### Clasificación inicial de documentos existentes

| Documento o familia | Clase durante 001 | Autoridad viva o función definitiva |
|---|---|---|
| `docs/ESQUEMA_MANIFIESTO.md` | `normative` hasta paridad; después `archive` | `specs/system/manifests.md` |
| `docs/CONTRATOS_OPERATIVOS_V2.md`, `docs/FASE3_EJECUCION.md` | `normative` hasta paridad; después `archive` | `specs/system/execution.md` y `specs/system/state-policy-routing.md` |
| `docs/ESQUEMA_CALIDAD_ROUTING.md`, `docs/FASE4_POLITICA.md` | `normative` hasta paridad; después `archive` | `specs/system/quality-research.md` y `specs/system/state-policy-routing.md` |
| `docs/FASE5_PANEL.md` | `normative` hasta paridad; después `archive` | `specs/system/surfaces.md` |
| `docs/IMPLEMENTACION_ROUTING_V2.md` | `guide`; nunca autoridad de conducta | `ROADMAP.md` |
| `docs/TRAZABILIDAD_ROUTING_V2.md` | `evidence`; nunca autoridad de conducta | `specs/anchors.json` y `docs/evidence/` |
| `README.md` | `guide` | Navegación; no sustituye specs vivas |
| `docs/medidas/**` | `evidence` | Medidas históricas inmutables |
| `docs/evidence/**` | `evidence` | V1 inmutable y V2 append-only; nunca autoridad normativa |
| `docs/examples/**` | `guide`/ejemplos versionados | Conservan su versión y apuntan al schema correspondiente |
| `docs/schemas/**` actuales | `normative` específico para su contrato versionado | Conservan autoridad de forma; una evolución abre schema nuevo |
| `docs/REGLAS_INGENIERIA_RUST.md` | `normative` específico | Autoridad vigente de reglas Rust y umbrales |
| `docs/DEUDA_MODULAR_RUST.md` | `evidence`/registro operativo específico | Autoridad vigente de la deuda modular heredada |

Cada ruta individual aparecerá en `docs/DOCUMENT_CLASSIFICATION.md`. La clase sólo cambia a `archive`
cuando `parity_verified: true`, todas las afirmaciones normativas tienen destino y prueba, y el enlace
de autoridad existe. Hasta entonces el documento anterior conserva su contenido y autoridad.

### Key Entities

- **Capacidad viva**: Unidad pública estable de Batuta; posee ID, estado, spec, requisitos,
  verificaciones, evidencia y trabajo futuro enlazado.
- **Paquete de cambio**: Registro histórico de una modificación acotada; contiene especificación,
  plan, tareas, checklists, impacto y evidencia de aceptación.
- **Anchor de requisito**: Relación verificable entre capacidad, requisito, rutas, pruebas, evidencia y
  roadmap.
- **Impacto de feature**: Declaración cerrada de la naturaleza del cambio, compatibilidad, migración,
  rollback y actualización de contratos vivos.
- **Registro de evidencia**: Prueba sellada de una transición o verificación, con procedencia explícita
  y compatibilidad de lectura con el legado.
- **Entrada de roadmap**: Slice permanente con dependencias, estado, criterios de aceptación,
  autorización externa requerida y enlaces a sus paquetes.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: El 100 % de las capacidades públicas actuales aparece exactamente una vez en el registro
  y puede recorrerse hasta su spec propietaria y su roadmap cuando no está completada. El denominador
  es el conjunto de IDs `CAP-*` declarado en las siete specs vivas, que debe ser idéntico al de
  `anchors.json`.
- **SC-002**: El 100 % de los requisitos marcados como implementados tiene al menos una verificación
  ejecutable existente; el 100 % de los parciales o externos tiene trabajo y protocolo enlazados. El
  denominador es el conjunto de IDs `REQ-*` de las specs vivas, sin duplicados y en biyección con los
  anchors de requisito.
- **SC-003**: La suite de mutación rechaza todos los casos previstos de duplicado, huérfano,
  referencia desconocida, verificación ausente, cambio funcional sin spec y refactor sin
  caracterización.
- **SC-004**: Los 19 registros V1 y todos sus snapshots conservan exactamente los bytes del baseline y
  el validador acepta tanto legado V1 como evidencia V2 anclada. El baseline es
  `7de68af2c9a36ba3dcc65971e4bba83231fb3855` y el manifest enumera exactamente siete rutas con tamaño
  y SHA-256: los seis artefactos V1 en `artifacts` y, separado en `run_records`, el registro de corrida
  `docs/evidence/baseline.json`. La separación es normativa: `baseline.json` documenta una corrida
  anterior (`head` `174b1279c5f35a78d808a30565df69a73ac9d887`) y **no** es uno de los seis artefactos
  V1, pero la clasificación lo declara `immutable` y por tanto su preservación se sella igual.
- **SC-005**: Una ejecución sin red de la entrada única de aceptación completa el 100 % de los gates y
  CI invoca esa misma entrada sin una lista divergente.
- **SC-006**: Una persona nueva puede partir de cualquiera de las capacidades registradas y localizar
  en menos de cinco minutos autoridad, estado, requisitos, verificación y siguiente slice sin hallar
  dos contratos normativos competidores.
- **SC-007**: Cero ficheros de producto o contratos públicos quedan modificados por este slice sin un
  requisito, impacto y verificación trazables.
- **SC-008**: La verificación de integración reporta cero ficheros administrados ausentes o modificados,
  cero rutas inválidas y la versión fijada por el repositorio. La fuente oficial es la salida JSON de
  `specify integration status --json`; el gate offline comprueba versión, manifests, rutas y hashes
  locales sin ejecutar Spec Kit ni acceder a red.

### Protocolos de medición

- **Navegación (SC-006)**: se cronometran por separado `CAP-MANIFESTS`, `CAP-STATE` y `CAP-ROLLOUT`.
  Para cada caso el reloj empieza con `specs/anchors.json` abierto y termina cuando el revisor muestra
  la spec propietaria, estado, requisito, verificación o evidencia y roadmap. Cada recorrido debe
  tardar como máximo cinco minutos y no encontrar otra autoridad normativa.
- **Rendimiento de validadores**: en un job `ubuntu-latest`, cada uno de los cinco procesos Python
  (`validate_spec_anchors.py`, `validate_tdd_evidence.py`, `check_modularity.py`,
  `check_architecture.py` y `check_speckit_integration.py`) se lanza por separado con timeout de cinco
  segundos. Se excluyen compilación/pruebas Rust, Node, `unittest` agregado y `local_gates.sh`.
- **Determinismo**: cada fixture se ejecuta dos veces con el mismo árbol y entorno sin secretos; se
  comparan byte a byte `stdout`, `stderr` y código de salida. Los diagnósticos se ordenan por código,
  ruta e ID antes de imprimirse.

## Assumptions

- El baseline K4 consolidado y verde es la fuente para reconstruir el inventario inicial, no evidencia
  de un ciclo TDD histórico. Su frontera exacta es
  `7de68af2c9a36ba3dcc65971e4bba83231fb3855` porque contiene los gates K4 verdes y los seis artefactos
  V1 que se sellarán antes de crear gobernanza V2.
- Las especificaciones y documentación operativa se mantienen en español; IDs y claves de contratos
  se mantienen en ASCII.
- La versión inicial de autoría permanece fijada en `v1.0.2`; actualizarla queda fuera de este slice.
- La ausencia de una instalación global de Spec Kit no relaja ningún gate: el checker permanente lee
  sólo los artefactos administrados del repositorio; la herramienta fijada se usa fuera del gate para
  autoría o para obtener el informe oficial cuando está disponible.
- No se ejecutan proveedores reales, red, promociones ni operaciones con coste durante esta adopción.
- Los slices 002–008 tendrán paquetes separados y actualizarán las specs vivas afectadas al integrarse.
