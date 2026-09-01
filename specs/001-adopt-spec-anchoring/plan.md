# Implementation Plan: Adopción spec-anchored

**Branch**: `sdd/spec-anchored-adoption` | **Date**: 2026-08-31 |
**Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/001-adopt-spec-anchoring/spec.md`

## Summary

Adoptar sobre el baseline K4 sellado un modelo SDD híbrido que mantenga contratos vivos por
capacidad y paquetes de cambio inmutables. El slice crea la constitución, inventario, roadmap,
trazabilidad cerrada, evidencia V2 compatible con V1 y validadores offline; no cambia el
comportamiento del producto. La aceptación queda centralizada en `scripts_ci/local_gates.sh`.

## Technical Context

**Language/Version**: Rust 1.98, edición 2024 para el producto; Python 3.11+, Bash 4.4+, Node 20+ y
Git 2.39+ para gates

**Primary Dependencies**: Biblioteca estándar de Python; JSON Schema draft 2020-12 como contrato;
Spec Kit v1.0.2 sólo para autoría; ninguna dependencia nueva de runtime

**Storage**: Markdown, JSON y JSONL versionados en Git; evidencia histórica direccionada por contenido

**Testing**: `unittest`, pruebas de mutación dirigidas, validación de fixtures, `cargo test` y gate local

**Target Platform**: `ubuntu-latest` en CI y Linux con las versiones mínimas anteriores; GNU
`sha256sum`/`timeout` para preservación y presupuesto temporal

**Project Type**: Workspace Rust con CLI y una capa de gobernanza/autoría fuera del binario

**Performance Goals**: Cada uno de los cinco validadores Python completa como proceso independiente en
menos de 5 segundos en `ubuntu-latest`. La medición excluye Rust, Node, `unittest` agregado y el gate
completo; el tiempo total continúa dominado por Clippy y las pruebas Rust

**Constraints**: Determinista significa mismos bytes de salida y código para fixtures idénticos;
offline y cero credenciales; evidencia V1 byte a byte sobre
`7de68af2c9a36ba3dcc65971e4bba83231fb3855`; contratos cerrados; diagnósticos ordenados; una sola
definición de gates; ningún efecto real de proveedor

**Scale/Scope**: 10 crates, unas 35 000 líneas Rust, 11 capacidades iniciales, 14 requisitos del slice,
19 registros de evidencia V1 y ocho entradas permanentes de roadmap

## Constitution Check

*GATE previo a Phase 0: PASS. Revalidado después de Phase 1: PASS.*

| Principio | Evidencia de cumplimiento del diseño |
|---|---|
| I. Nada se declara, se demuestra | Cada estado implementado enlaza verificaciones; mutaciones prueban que los gates fallan |
| II. Especificación, pruebas y código evolucionan juntos | `FeatureImpactV1` y anchors hacen explícitas las capacidades y specs afectadas |
| III. TDD y evidencia verificable | Las pruebas de los validadores se escriben rojas y `EvidenceRecordV2` conserva red/green/mutación |
| IV. Modularidad y dependencias hacia el dominio | El gate usa umbrales y excepciones cerradas con extracción y caracterización concretas |
| V. Contratos cerrados y compatibilidad explícita | Los tres contratos tienen versión, claves exactas y rechazo de campos desconocidos |
| VI. Estado durable, atómico y recuperable | Staging, backup y reintento conservan el activo ante migración/publicación fallida; los snapshots son inmutables |
| VII. CI offline y efectos externos autorizados | Los gates usan sólo fixtures; Spec Kit y proveedores reales no se ejecutan en CI; toda autorización externa futura fija alcance, rutas, límites, presupuesto, caducidad y rollback |

Los siete principios se citan con el numeral y el título exactos de
[`.specify/memory/constitution.md`](../../.specify/memory/constitution.md) v1.0.0, de modo que una
enmienda que renombre, reordene o inserte un principio produzca un desajuste visible en esta tabla.

No hay violaciones ni excepciones constitucionales para este slice.

## Project Structure

### Documentation (this feature)

```text
specs/001-adopt-spec-anchoring/
├── spec.md
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── feature-impact.json
├── contracts/
│   └── README.md
├── checklists/
│   ├── requirements.md
│   ├── acceptance.md
│   └── acceptance-evidence.md
└── tasks.md
```

### Repository changes

```text
.agents/skills/speckit-*/        integración Codex administrada
.specify/                        versión, templates, scripts y constitución
AGENTS.md                        instrucciones breves para agentes
CONTRIBUTING.md                  flujo humano spec-anchored
ROADMAP.md                       spec of specs con RM-001..RM-008
specs/
├── README.md                    autoridad y ciclo de vida
├── anchors.json                 SpecAnchorRegistryV1
├── schemas/                     contratos vivos cerrados
└── system/                      specs vivas por capacidad
docs/
├── adr/ADR-0001-*.md            decisión de gobernanza
├── evidence/                    V1 intacto, manifest de preservación y V2
└── DOCUMENT_CLASSIFICATION.md   clasificación y supersesión
scripts_ci/
├── validate_spec_anchors.py
├── validate_tdd_evidence.py
├── check_modularity.py
├── check_architecture.py
├── check_speckit_integration.py
├── modularity_exceptions.json
├── local_gates.sh
└── tests/                       pruebas y mutaciones de gates
.github/workflows/ci.yml         invoca sólo el gate local
```

El código de producto bajo `crates/` no cambia en 001. Los validadores viven en `scripts_ci/` porque
son la frontera de aceptación del repositorio; los contratos vivos y su registro viven en `specs/`.

## Phase 0: Research

Las decisiones y alternativas están consolidadas en [research.md](research.md). No quedan decisiones
pendientes.

## Phase 1: Design & Contracts

- [data-model.md](data-model.md) define entidades, validaciones y transiciones.
- [contracts/README.md](contracts/README.md) fija contratos, rutas vivas y códigos de salida.
- [quickstart.md](quickstart.md) describe la validación end-to-end y los resultados esperados.
- [research.md](research.md#matriz-mínima-de-mutaciones) fija mutaciones, diagnósticos y códigos.
- [checklists/acceptance-evidence.md](checklists/acceptance-evidence.md) demuestra la calidad de los 36
  criterios sin asumir la aprobación del revisor.

La revisión post-diseño mantiene todos los gates constitucionales en PASS: las decisiones no añaden
dependencias de runtime, no reescriben evidencia, no abren red y hacen trazable cada excepción.

## Implementation Strategy

1. Presentar la matriz 36/36; sólo tras aprobación humana, cambiar únicamente sus 36 marcas y repetir
   `$speckit-analyze` verde para habilitar T001.
2. Escribir primero fixtures y mutaciones rojas para anchors, evidencia, modularidad y arquitectura.
3. Implementar validadores pequeños con biblioteca estándar y claves exactas.
4. Crear specs vivas, roadmap, ADR, esquemas y registro hasta que la estructura quede verde.
5. Sellar los bytes V1 antes de añadir un log V2 independiente.
6. Clasificar documentos; si la paridad no es total, conservar la autoridad anterior sin cambios.
7. Publicar migraciones/estado desde staging; ante fallo conservar activo, backup y reintento idempotente.
8. Integrar todos los validadores en `local_gates.sh`; CI usa historial completo, exige base Git y llama
   únicamente esa entrada.
9. Ejecutar convergencia, gates completos, checker offline e informe oficial de integración Spec Kit.

## Medición de aceptación

- La cobertura se calcula por igualdad de conjuntos entre IDs `CAP-*`/`REQ-*` declarados en las siete
  specs vivas y los registrados en `anchors.json`, no por muestreo.
- La navegación se demuestra por separado con `CAP-MANIFESTS`, `CAP-STATE` y `CAP-ROLLOUT`, desde
  `anchors.json` hasta spec, estado, requisito, verificación/evidencia y roadmap en ≤ 5 minutos cada uno.
- Las mutaciones mínimas cubren anchors, impacto y Git, V1/V2, modularidad, arquitectura e integridad
  de Spec Kit con el diagnóstico/código exacto de `research.md`.
- `specify integration status --json` es la fuente oficial de SC-008. El checker de CI sólo valida
  versión `1.0.2`, manifests, rutas y hashes locales, sin ejecutar Spec Kit ni usar red.
- La aprobación de `checklists/acceptance.md` es humana: esta fase prepara 36 filas de evidencia y no
  cambia ninguna marca. Tras una única aprobación se modificarán exclusivamente los 36 `[ ]` a `[x]`.

## Complexity Tracking

No se registran violaciones constitucionales. Las cinco excepciones modulares iniciales son deuda
heredada explícita y no una excepción de este diseño; incluyen extracción y pruebas y se cerrarán en
RM-002..RM-005.
