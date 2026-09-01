<!--
Sync Impact Report
- Version change: template sin ratificar -> 1.0.0
- Added principles:
  - I. Nada se declara, se demuestra
  - II. Especificación, pruebas y código evolucionan juntos
  - III. TDD y evidencia verificable
  - IV. Modularidad y dependencias hacia el dominio
  - V. Contratos cerrados y compatibilidad explícita
  - VI. Estado durable, atómico y recuperable
  - VII. CI offline y efectos externos autorizados
- Added sections:
  - Restricciones de ingeniería
  - Flujo spec-anchored y gates
- Removed sections: ninguna; el fichero anterior era la plantilla sin adoptar.
- Follow-up TODOs: ninguno.
-->
# Constitución de Batuta

## Core Principles

### I. Nada se declara, se demuestra

Toda afirmación operativa DEBE apoyarse en evidencia observable y reproducible. Un manifiesto,
un nombre de ruta, una respuesta plausible o una configuración inspeccionada no demuestran que
una capacidad se ejecutó. Los recibos, estados de promoción y decisiones de routing DEBEN derivarse
de hechos medidos, con procedencia y vigencia explícitas. Un resultado ausente o ambiguo nunca se
convierte silenciosamente en éxito.

### II. Especificación, pruebas y código evolucionan juntos

Las especificaciones vivas de `specs/system/` DEBEN describir el comportamiento público vigente.
Todo cambio de comportamiento o contrato DEBE actualizar en el mismo cambio su especificación viva,
sus pruebas y su código. Una divergencia entre esos tres artefactos es un defecto bloqueante: ninguno
gana silenciosamente. Los paquetes `specs/NNN-*` integrados son evidencia histórica inmutable; una
evolución posterior abre un paquete nuevo y conserva los anteriores.

### III. TDD y evidencia verificable

Todo comportamiento nuevo y toda corrección DEBEN comenzar con una prueba roja reproducible, seguir
el ciclo red-green-refactor y registrar evidencia vinculada a requisitos estables. Los refactors sin
cambio funcional DEBEN comenzar con caracterización de API, mensajes, serialización, hashes y bytes
relevantes. El trabajo histórico reconstruido DEBE identificarse como `reconstructed_audit` y nunca
presentarse como TDD retroactivo. Ningún porcentaje de cobertura sustituye las pruebas de invariantes,
errores, límites, recuperación y concurrencia.

### IV. Modularidad y dependencias hacia el dominio

Cada módulo DEBE tener una responsabilidad y un motivo principal para cambiar. Las dependencias DEBEN
apuntar hacia contratos y dominio; `batuta-contract` no puede depender de otro crate interno y el
dominio no puede depender de CLI, TUI ni adaptadores concretos. La API pública DEBE ser menor que la
implementación. Las reglas detalladas, umbrales y excepciones se rigen por
[`docs/REGLAS_INGENIERIA_RUST.md`](../../docs/REGLAS_INGENIERIA_RUST.md) y no se duplican aquí.

### V. Contratos cerrados y compatibilidad explícita

Todo contrato persistido o externo DEBE tener versión, rechazar campos desconocidos y validarse en un
único borde compartido por CLI, MCP, TUI, ficheros y pruebas. Los IDs, vocabularios, estados, unidades y
errores públicos DEBEN ser tipos cerrados y estables. Los hashes y sellos DEBEN usar una representación
canónica. Todo cambio incompatible DEBE declarar migración, convivencia, rollback y ventana de
deprecación; una retirada anticipada requiere una exención humana explícita.

### VI. Estado durable, atómico y recuperable

El estado operativo DEBE publicarse mediante una única frontera durable con staging, hash base, CAS,
confirmación y escritura atómica. Una caída no puede dejar un documento activo parcial ni perder la
distinción entre «no ocurrió» y «resultado desconocido». Las migraciones DEBEN conservar backup
recuperable, lectura histórica e idempotencia. Los resultados ambiguos DEBEN preservar recibos y
reservas y bloquear cualquier promoción automática.

### VII. CI offline y efectos externos autorizados

Las pruebas y gates permanentes DEBEN ser deterministas y funcionar sin red, credenciales ni cuota de
proveedores. Ejecutables, reloj, sleeper y servicios externos se sustituyen por dobles controlados en
CI. Red, proveedores reales, despliegue, promociones, costes u otros efectos externos sólo pueden
ejecutarse con una autorización independiente que fije alcance, rutas, límites, presupuesto,
caducidad y rollback. La capacidad técnica o un grant genérico no equivalen a esa autorización.

## Restricciones de ingeniería

- La documentación normativa y operativa se escribe en español; las claves de esquemas y los IDs
  estables usan ASCII.
- Los requisitos usan `REQ-<CAPACIDAD>-NNN`, las capacidades `CAP-*` y las decisiones `ADR-NNNN`.
- Todo documento persistido público DEBE tener un JSON Schema cerrado y ejemplos válidos cuando su
  formato no sea trivial.
- Spec Kit es una herramienta de autoría fijada por versión; no forma parte del binario ni de su
  ejecución. Actualizarla requiere un cambio revisado independiente.
- La deuda heredada no obliga a reescribir todo el sistema, pero cada zona tocada DEBE quedar igual o
  más clara y cualquier excepción debe quedar registrada con extracción y caracterización concretas.

## Flujo spec-anchored y gates

Cada slice funcional DEBE recorrer `specify -> clarify -> plan -> checklist -> tasks -> analyze ->
implement -> converge`. `analyze` debe estar verde antes de modificar código y `converge` se repite
hasta que no añada tareas. Si durante la implementación aparece un comportamiento no previsto, el
código se detiene, se actualiza la especificación y se repiten planificación y análisis.

Toda propuesta DEBE incluir un `FeatureImpactV1` que clasifique el cambio y enlace capacidades,
requisitos, compatibilidad, migración, rollback y actualización de specs vivas. Los requisitos
implementados necesitan una prueba o gate ejecutable; los parciales o externos necesitan una entrada
de roadmap y protocolo. `scripts_ci/local_gates.sh` es la única definición ejecutable de aceptación y,
como mínimo, verifica formato, `no_std`, anchors, evidencia TDD, modularidad, arquitectura, sidecar
offline, Clippy con todos los targets/features y todas las pruebas del workspace.

Una revisión no puede declarar completado un slice mientras haya requisitos sin evidencia, campos
desconocidos aceptados, documentos normativos duplicados o diferencias no explicadas entre spec,
prueba y código.

## Governance

Esta constitución prevalece sobre guías, planes y prácticas incompatibles del repositorio. Una
modificación normativa requiere aprobación humana explícita, motivación, impacto de migración y un
incremento SemVer de esta constitución: MAJOR para retirar o redefinir garantías, MINOR para añadir o
ampliar obligaciones y PATCH para aclaraciones no semánticas. La fecha de ratificación no cambia; la
fecha de enmienda se actualiza con cada modificación.

Cada plan y revisión DEBE comprobar la constitución antes de implementar y antes de integrar. Las
excepciones son temporales, nominativas y verificables: deben indicar requisito afectado, riesgo,
responsable, caducidad y criterio de cierre. El historial de Spec Kit y `docs/evidence/` no se reescribe
para aparentar cumplimiento posterior.

**Version**: 1.0.0 | **Ratified**: 2026-08-31 | **Last Amended**: 2026-08-31
