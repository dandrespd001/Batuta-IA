# Contribuir a Batuta

## Antes de editar

Empieza en [`specs/README.md`](specs/README.md): localiza la capacidad en
[`specs/anchors.json`](specs/anchors.json), abre su spec propietaria y sigue el `roadmap_id`. Lee
después la [constitución](.specify/memory/constitution.md) y las
[instrucciones para agentes](AGENTS.md). Las guías no sustituyen las specs vivas.

Comprueba que la herramienta de autoría es la fijada sin actualizar una instalación global:

```sh
uvx --from git+https://github.com/github/spec-kit.git@v1.0.2 specify --version
```

El comando requiere red o una caché ya disponible y debe mostrar `1.0.2`. No forma parte de los
gates permanentes.

## Flujo humano

1. Acota un solo slice y crea un paquete `specs/NNN-nombre/` enlazado desde `ROADMAP.md`.
2. Recorre `specify`, `clarify`, `plan`, `checklist`, `tasks` y `analyze`. No implementes hasta que el
   checklist humano aplicable esté aprobado y el análisis quede verde.
3. Publica `feature-impact.json` y comprueba capacidades, requisitos, compatibilidad, migración,
   rollback, caracterización y specs vivas afectadas.
4. Escribe y ejecuta primero las pruebas o mutaciones rojas. Implementa el mínimo cambio, deja todo
   verde y refactoriza conservando los mismos resultados observables.
5. Repite `analyze` si el alcance cambia y `converge` hasta que no aparezcan contradicciones ni tareas.
6. Ejecuta `./scripts_ci/local_gates.sh`. Revisa el diff, la clasificación documental y la evidencia
   antes de marcar tareas o cambiar el estado del roadmap.

Detente si aparece conducta no especificada, otra autoridad normativa, un contrato incompatible sin
migración, una ruta sin capacidad, una base Git ausente, una prueba roja que no falla por el motivo
esperado o una aprobación pendiente. Corrige primero spec, plan y alcance; no rebajes el gate.

## Plantilla `FeatureImpactV1`

El schema normativo es
[`specs/schemas/feature-impact-v1.schema.json`](specs/schemas/feature-impact-v1.schema.json). Esta
plantilla es estructuralmente completa; sustituye el feature y los IDs por valores existentes y
mantén todos los arrays ordenados:

```json
{
  "schema_version": 1,
  "feature_id": "002-nombre-del-cambio",
  "change_type": "contract",
  "capabilities": [
    "CAP-CONTRACTS"
  ],
  "requirements": [
    "REQ-CONTRACTS-001"
  ],
  "compatibility": {
    "public_contract": "compatible",
    "persisted_data": "not_applicable",
    "notes": "Justificación verificable de ambos veredictos."
  },
  "migration": {
    "required": false,
    "plan": null,
    "backup": null,
    "retry": null
  },
  "rollback": {
    "strategy": "revert",
    "procedure": "Procedimiento concreto para volver al estado anterior.",
    "success_check": "Comprobación observable de que la recuperación terminó."
  },
  "living_specs_updated": true,
  "characterization": []
}
```

La precedencia es `contract > behavior > internal_refactor > docs_only`. `contract` y `behavior`
exigen `living_specs_updated: true`; los otros dos, `false`. Un refactor interno exige al menos una
ruta de caracterización existente. Si cualquier compatibilidad es `incompatible`, la migración es
obligatoria y `plan`, `backup` y `retry` son textos no vacíos. Sólo `docs_only` admite rollback
`not_applicable`, con procedimiento y comprobación nulos.

## Recuperación

Una migración o publicación multiarchivo se prepara fuera del estado activo. Valida el staging,
conserva un backup restaurable y demuestra que repetir la misma entrada es idempotente. Publica de
forma coordinada sólo con todos los gates verdes. Si algo falla, el activo conserva sus bytes, el
backup se restaura y el borrador queda disponible para reintento. Una supersesión sin paridad total
deja la autoridad anterior intacta.

## Autorización externa

Antes de usar red, credenciales, proveedores, despliegues, promociones o presupuesto, registra una
aprobación humana independiente con estos campos, todos obligatorios:

```text
alcance: sistemas y operación exactos autorizados
rutas: endpoints, repositorios, entornos y destinos permitidos
límites operativos: concurrencia, volumen, tiempo y condiciones de parada
presupuesto: importe y moneda; cero si no puede existir coste
caducidad: fecha y hora absoluta con zona horaria
rollback: pasos, responsable y comprobación de recuperación
```

La autorización no se hereda entre slices ni se amplía por inferencia. Si falta un dato, expiró o el
efecto excede sus límites, no se ejecuta. Los tests, dobles y gates offline siguen disponibles sin esa
autorización.
