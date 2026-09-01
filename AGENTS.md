# Instrucciones para agentes

## Autoridad y alcance

Antes de cambiar Batuta, leer la constitución en `.specify/memory/constitution.md`, la capacidad
afectada en `specs/system/`, su entrada en `specs/anchors.json` y el slice correspondiente de
`ROADMAP.md`. La precedencia es: constitución, spec viva propietaria, schema vivo y, por último,
guías o historia. `README.md`, ADR, ejemplos, roadmap y paquetes integrados ayudan a navegar o
auditar, pero no sustituyen la obligación normativa vigente.

No ampliar el alcance por conveniencia. En el checkpoint T032–T037 de 001 no está autorizado cambiar
`crates/`, reescribir evidencia V1 ni ejecutar proveedores, promociones u operaciones con coste.

## Ciclo spec-anchored

Todo slice recorre `specify -> clarify -> plan -> checklist -> tasks -> analyze -> implement ->
converge`. Crear o actualizar `specs/NNN-*/feature-impact.json` antes del producto. `analyze` debe
estar verde antes de implementar y `converge` se repite hasta no descubrir contradicciones ni tareas.
La autoridad viva, anchors, roadmap, pruebas y código cambian juntos cuando corresponda.

Detener la implementación y volver a la spec si aparece conducta no prevista, una segunda autoridad,
un contrato o dato persistido no declarado, una base Git no resoluble, una aprobación humana
pendiente o un requisito sin verificación. No marcar tareas por intención ni declarar completado un
roadmap con gates o evidencia pendientes.

## TDD, recuperación y verificación

Escribir primero una prueba roja reproducible; observar el fallo; implementar el mínimo cambio verde;
refactorizar sin perder caracterización. Los refactors empiezan con pruebas de igualdad observable.
No presentar auditoría reconstruida como TDD contemporáneo. La evidencia nueva usa V2; los siete
ficheros sellados y los 19 registros V1 son inmutables.

Preparar cambios multiarchivo y migraciones en staging. Publicar sólo después de validar schema,
paridad, backup, reintento idempotente y rollback. Ante un fallo, conservar el activo íntegro y dejar
el borrador recuperable; nunca comunicar un estado parcial como activo.

La única entrada agregada de aceptación es:

```sh
./scripts_ci/local_gates.sh
```

No duplicar su lista en CI ni omitir un gate. Los gates son offline. Para compilación local se
mantienen prioridad baja y dos jobs por defecto; `BATUTA_BUILD_JOBS` sólo es un override consciente.

## Efectos externos

No usar red, credenciales, proveedores reales, despliegues, promociones, recursos con coste ni
mutaciones externas sin autorización humana independiente y vigente. Debe fijar por escrito alcance,
rutas, límites operativos, presupuesto, caducidad y rollback. Una capacidad técnica, un grant o una
autorización anterior no equivalen a permiso. Si falta un campo o la autorización caducó, detener el
efecto; los gates offline continúan siendo obligatorios.
