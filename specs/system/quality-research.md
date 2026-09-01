# Calidad y research vigentes

## CAP-QUALITY — Calidad verificable

**Estado**: `implemented`. **Roadmap**: `RM-003` sólo divide el modelo sin cambiar resultados.

### REQ-QUALITY-001 — Evidencia compatible por ruta y acción

Las observaciones contribuyen únicamente cuando ruta/revisión, benchmark, versión, escenario,
configuración, scaffold y métrica coinciden y siguen vigentes. La proyección por acción expone
puntaje investigado, cobertura, rango, edad, procedencia y exclusiones cerradas; no mezcla revisiones
ambiguas ni fabrica verificación a partir del fabricante.

### REQ-QUALITY-002 — Overrides append-only

Los overrides `set`/`clear` son eventos ordenados y append-only. Un `set` conserva el valor
investigado sustituido y nunca crea verificación; un `clear` recupera el valor investigado sin borrar
historia. Hash y resultado no dependen del orden de entrada equivalente.

## CAP-RESEARCH — Investigación de calidad

**Estado**: `partial`. **Roadmap**: `RM-005`.

### REQ-RESEARCH-001 — Propuestas selladas y activación confirmada

Una propuesta ordena y sella bases, observaciones y fuentes primarias completas, queda en staging y
no cambia evidencia activa. Aplicar exige confirmación y que manifest, evidencia base y contenido no
hayan cambiado; la ruta investigadora no puede autocertificarse.

### REQ-RESEARCH-002 — Update síncrono de extremo a extremo

**Estado parcial**: CLI y TUI parsean y encolan la solicitud y el almacén soporta staging/status/apply,
pero `research update` todavía no ejecuta síncronamente consulta, normalización y propuesta completa.
La aceptación pertenece a `RM-005`.

## Protocolo manual para REQ-RESEARCH-002

Precondiciones: perfil web-capable aprobado, fuentes primarias controladas y paquete de `RM-005`.
Ejecutar `research update` para una ruta y una acción, observar consulta, normalización, sello y
staging, y repetir con la propia ruta como única fuente. Se acepta si la primera llamada termina con
una propuesta completa sin activar evidencia y la autocertificación se rechaza; repetir la misma
entrada no crea una activación ni una propuesta divergente.
