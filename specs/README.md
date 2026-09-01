# Autoridad de especificaciones de Batuta

`specs/system/` contiene la única autoridad viva sobre el comportamiento público vigente. Cada
capacidad aparece exactamente una vez como `CAP-*` y cada obligación exactamente una vez como
`REQ-<CAPACIDAD>-NNN`. El inventario navegable está en [`anchors.json`](anchors.json), su forma en
[`spec-anchor-registry-v1.schema.json`](schemas/spec-anchor-registry-v1.schema.json) y el trabajo
futuro en [`ROADMAP.md`](../ROADMAP.md).

La precedencia es: constitución, spec viva propietaria, schema vivo y, por último, guías o historia.
Un inventario, roadmap, ADR, ejemplo o registro de evidencia ayuda a navegar o demostrar, pero no
sustituye a la spec propietaria.

## Navegación

1. Buscar el ID `CAP-*` en `anchors.json`.
2. Abrir `owner_spec` y localizar allí la misma capacidad y todos sus `REQ-*`.
3. Seguir cada verificación ejecutable o protocolo y la evidencia registrada.
4. Si `roadmap_id` no es nulo, abrir la entrada permanente correspondiente en `ROADMAP.md`.

Las capacidades `implemented` tienen todos sus requisitos activos implementados y al menos una
prueba o gate por requisito. Las `partial` mezclan una parte local observable con trabajo incompleto;
las `external` dependen enteramente de autorización o sistemas externos; las `deprecated` sólo
conservan compatibilidad y una ruta de retirada. Un protocolo manual nunca convierte un requisito en
implementado.

## Ciclo de vida

Las specs vivas cambian sólo mediante un paquete `specs/NNN-*` aprobado. Mientras se implementa, el
paquete enlaza capacidades, requisitos, compatibilidad y recuperación mediante `feature-impact.json`.
Al integrarse, ese paquete queda como historia inmutable; el estado vigente continúa en
`specs/system/`. Si aparece conducta no prevista, se detiene el código, se actualiza la spec y se
repiten planificación y análisis.

El inventario inicial apunta al log V1 de K4 sin alterar su procedencia: cada una de sus 19 filas
conserva el `evidence_mode` real (15 `tdd` y 4 `reconstructed_audit`). El registro no convierte una
auditoría reconstruida en TDD retroactivo.

## Validación

```sh
python3 scripts_ci/validate_spec_anchors.py
```

El MVP valida offline estructura cerrada, estados, cobertura biyectiva, rutas, roadmap y selectores.
La correlación con un diff Git y `--base` pertenece a T021 y no forma parte de este checkpoint.
