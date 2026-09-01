# ADR-0001 — Gobernanza spec-anchored

- **Estado**: aceptada
- **Fecha**: 2026-08-31
- **Decisores**: revisión humana del paquete 001

## Contexto

Batuta llegó a K4 con contratos implementados y evidencia útil, pero repartidos entre documentos de
fases, esquemas, guías, código y pruebas. Esa duplicación hacía posible encontrar dos afirmaciones
normativas sobre una misma capacidad y no distinguía con suficiente claridad autoridad vigente,
verificación, evidencia y snapshot histórico.

## Decisión

Se adopta persistencia híbrida:

- `specs/system/` es la autoridad viva y única de conducta pública;
- `specs/anchors.json` enlaza capacidades y requisitos con pruebas, evidencia y roadmap, pero no
  sustituye a la spec propietaria;
- `ROADMAP.md` conserva IDs permanentes para trabajo incompleto y sólo enlaza un paquete cuando existe;
- cada `specs/NNN-*` es mutable mientras se implementa y pasa a historia inmutable al integrarse;
- constitución, spec propietaria, schema vivo y guía/historia forman la precedencia normativa;
- documentos anteriores sólo pasan de `normative` a `archive` después de una matriz completa por
  sección, con requisito y verificación para cada fila;
- evidencia y medidas nunca se superseden destructivamente. El inventario K4 conserva el
  `evidence_mode` real de cada registro y no convierte `reconstructed_audit` en TDD retroactivo.

El estado vivo se publica como una unidad: spec, anchors y roadmap deben validar juntos. Si falla una
ruta, schema, paridad o verificación, la autoridad anterior continúa activa y el borrador queda
recuperable. El rollback del paquete 001 es revertir el conjunto de gobernanza, no reescribir V1.

## Consecuencias

Una persona puede partir de un `CAP-*` y llegar a una única spec, sus `REQ-*`, verificaciones,
evidencia y siguiente slice. Los requisitos parciales o externos ya no pueden ocultarse como
implementados. A cambio, todo cambio normativo necesita actualizar en conjunto la spec viva y su
declaración de impacto, y las matrices de paridad deben mantenerse verificables cuando se retire una
autoridad anterior.

Las seis autoridades heredadas auditadas en T016 pasan a archivo conservando íntegro su contenido
histórico; la clasificación y las matrices que justifican la transición viven en
[`DOCUMENT_CLASSIFICATION.md`](../DOCUMENT_CLASSIFICATION.md).
