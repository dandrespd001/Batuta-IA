# Contracts: adopción spec-anchored

Este directorio describe el contrato histórico del cambio 001. Al integrarse, las formas normativas
vivas quedan en `specs/schemas/`; este paquete no se actualiza para cambios posteriores.

## Contratos persistidos

| Contrato | Ruta viva | Instancia inicial |
|---|---|---|
| `SpecAnchorRegistryV1` | `specs/schemas/spec-anchor-registry-v1.schema.json` | `specs/anchors.json` |
| `FeatureImpactV1` | `specs/schemas/feature-impact-v1.schema.json` | `specs/001-adopt-spec-anchoring/feature-impact.json` |
| `EvidenceRecordV2` | `specs/schemas/evidence-record-v2.schema.json` | `docs/evidence/tdd-v2.jsonl` |

Todos declaran JSON Schema draft 2020-12, versión explícita y `additionalProperties: false` en cada
objeto. Las invariantes entre ficheros están en [data-model.md](../data-model.md).

Los campos y enums allí enumerados son exactos. Una prueba o gate ejecutable es una verificación; un
protocolo manual no lo es, y una evidencia sólo registra el resultado de una verificación ya
realizada. Un snapshot es el input normativo inmutable de esa evidencia. La procedencia
`reconstructed_audit` nunca se interpreta como TDD contemporáneo.

## Contratos de validadores

### `validate_spec_anchors.py`

```text
python3 scripts_ci/validate_spec_anchors.py [--base <git-ref>] [--report]
```

- Sin flags valida contratos, unicidad, rutas, cobertura y dependencias de roadmap.
- `--base` añade correlación entre el diff y los paquetes de impacto.
- `--report` informa deriva del diff sin convertirla en error; los errores de contrato siguen
  bloqueando.

La base tiene tres resultados cerrados:

| Entrada | Comportamiento | Código si la estructura es válida |
|---|---|---:|
| No se pasa `--base` ni `BATUTA_SPEC_BASE` | Valida toda la estructura, emite una sola vez `[GIT_DIFF_OMITTED] base Git no proporcionada; correlación de diff omitida` y no ejecuta diff | `0` |
| Se pasa una base resoluble | Valida estructura y correlaciona `git diff <base>...HEAD` | `0` |
| Se pasa una base explícita inválida, no presente en clon superficial o fuera de un repositorio Git | No degrada a modo estructural; emite `[GIT_BASE_UNRESOLVABLE] <ref>` | `2` |

Los fallos de invariantes encontrados antes o durante una correlación resoluble usan código `1`. CI
hace checkout con `fetch-depth: 0`, exige `BATUTA_SPEC_BASE` no vacío y resoluble y pasa esa variable a
la misma `scripts_ci/local_gates.sh`; no replica la lista de gates.

### `validate_tdd_evidence.py`

```text
python3 scripts_ci/validate_tdd_evidence.py
```

Valida primero el manifest de bytes V1, luego los 19 registros legados con sus reglas exactas y por
último cada registro V2 contra anchors, tasks y snapshot sellado.

### `check_modularity.py`

```text
python3 scripts_ci/check_modularity.py
```

Advierte a partir de 400 líneas. Falla desde 500 en producción o 700 en tests si no existe una
excepción válida. También falla por excepciones duplicadas, incompletas, obsoletas o con rutas ausentes.

### `check_architecture.py`

```text
python3 scripts_ci/check_architecture.py
```

Lee los manifests del workspace, construye sólo dependencias locales y falla por ciclos,
dependencias internas desde `batuta-contract` o dependencias desde dominio hacia `batuta-cli`.

### `check_speckit_integration.py`

```text
python3 scripts_ci/check_speckit_integration.py
```

Es el checker permanente offline. Lee `.specify/integration.json` y los manifests administrados,
exige versión `1.0.2`, integración `codex`, rutas relativas válidas, presencia de cada fichero y
SHA-256 exacto. No importa ni ejecuta Spec Kit, no consulta instalación global y no accede a red.

La fuente oficial de `SC-008` es un informe obtenido separadamente con:

```text
specify integration status --json
```

Ese informe debe declarar `status: ok`, versión `1.0.2`, integración `codex` y cero rutas/ficheros
ausentes, modificados o inválidos. Puede requerir la copia fijada ya cacheada y no forma parte de CI;
su indisponibilidad no relaja ni omite el checker offline.

## Salidas

Todos los validadores usan el mismo contrato de proceso:

| Código | Significado |
|---:|---|
| `0` | Contrato válido; puede haber advertencias documentadas |
| `1` | Una o más invariantes verificables fallaron |
| `2` | Invocación inválida o imposibilidad de leer el repositorio |

Los diagnósticos se escriben en `stderr`, incluyen ruta/ID concreto, nunca secretos y se ordenan de
forma determinista. Cada línea usa `[CODIGO_ESTABLE] ruta#id: detalle`, con rutas relativas y orden por
`(codigo, ruta, id, detalle)`. Los validadores no imprimen contenido completo de ficheros, variables de
entorno, tokens ni credenciales. Los warnings también van a `stderr`; un resumen de éxito único va a
`stdout`.

Determinismo significa que dos ejecuciones sobre el mismo fixture, árbol y argumentos producen
exactamente los mismos bytes en `stdout` y `stderr` y el mismo código, sin timestamps, orden de hash o
rutas absolutas variables. Un error JSON/sintáctico, referencia rota, hash incorrecto o invariante
incumplida es código `1`; sólo argumentos inválidos, base Git explícita no resoluble o imposibilidad de
leer la raíz del repositorio son código `2`.

## Presupuesto temporal

En `ubuntu-latest`, cada proceso se mide por separado y debe terminar antes del timeout de 5 segundos:

```text
timeout 5s python3 scripts_ci/validate_spec_anchors.py
timeout 5s python3 scripts_ci/validate_tdd_evidence.py
timeout 5s python3 scripts_ci/check_modularity.py
timeout 5s python3 scripts_ci/check_architecture.py
timeout 5s python3 scripts_ci/check_speckit_integration.py
```

La medición usa un checkout ya disponible y no incluye `unittest`, Rust, Node ni la ejecución agregada
de `local_gates.sh`. Un timeout es fallo de aceptación, no código contractual emitido por el validador.

## Recuperación contractual

Una migración sólo publica después de validar staging, backup y reintento. Si falla, el estado activo
permanece byte a byte, el backup se puede restaurar y repetir la misma entrada es idempotente. Una
supersesión sin paridad total conserva el documento anterior como autoridad y no lo reduce a enlace.
Una publicación multiarchivo fallida conserva el activo y el borrador recuperable; nunca comunica un
estado parcialmente publicado como éxito.

La lista mínima de mutaciones, sus códigos de diagnóstico y salidas esperadas está en
[research.md](../research.md#matriz-mínima-de-mutaciones). Todos los casos deben ejecutarse al menos dos
veces para comprobar orden y bytes estables.
