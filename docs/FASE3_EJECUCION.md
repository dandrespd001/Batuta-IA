# SPEC v2 — Orquestación, salud y relevo

> **Archivo histórico desde 2026-08-31.** La autoridad vigente es
> [`specs/system/execution.md`](../specs/system/execution.md), con estado generacional en
> [`specs/system/state-policy-routing.md`](../specs/system/state-policy-routing.md). La paridad está en
> [`DOCUMENT_CLASSIFICATION.md`](DOCUMENT_CLASSIFICATION.md); el contenido siguiente queda íntegro.

**Estado: aprobado para implementación.** Este documento reemplaza el contrato de Fase 3;
los recibos de canario existentes conservan compatibilidad de lectura.

## 1. Máquina de estados

Una ejecución transita de forma determinista:

```
Planned → Running → Completed
                  ↘ RetrySameRoute
                  ↘ Checkpointed → FallbackSelected → Running
                  ↘ BlockedByHarness
```

No hay llamadas especulativas paralelas. Sólo existe una ruta activa por ejecución.

## 2. Salud durable

`RouteHealth` conserva éxito reciente, latencia p95, fallos consecutivos, cooldown,
categoría del último fallo y próxima sonda. La clasificación procede de la respuesta del
harness, nunca de consultar saldo o claves.

- `Retry-After` corto: reanudar la misma ruta.
- cuota agotada: checkpoint y fallback inmediato;
- MiniMax sin plazo: sondas a 15, 30 y 60 minutos y luego cada hora;
- autenticación o saldo: fallback y bloqueo hasta intervención del harness.

Actualizar salud es una función pura sobre estado + evento + hora. La persistencia usa el
mismo guardado atómico que la política.

## 3. HandoffCheckpoint

El relevo usa JSON versionado y acotado:

```json
{
  "schema_version": 1,
  "objective": "…",
  "constraints": ["…"],
  "decisions": ["…"],
  "files": ["src/lib.rs"],
  "diff_summary": "…",
  "tests": [{"command":"cargo test","status":"failed","summary":"…"}],
  "failure": {"category":"quota_exhausted","message":"…"},
  "next_step": "…",
  "remaining_budget": {"tokens":12000,"wall_seconds":900}
}
```

No se reenvía el historial completo y no se hace una llamada adicional sólo para resumir.
Objetivo, fallo y siguiente paso no pueden estar vacíos. Rutas de fichero son relativas.

## 4. Recibo de routing

Cada decisión y relevo conserva:

- petición resuelta y perfil de acción;
- ruta y alias resuelto;
- calidad investigada, override, efectiva, cobertura y verificación;
- `evidence_hash` y `policy_hash`;
- petición resuelta, proyecciones evaluadas, descartes, autorizaciones y ruta;
- transiciones ordenadas y último `HandoffCheckpoint`, sin reenviar el historial;
- coste estimado y razones de descarte;
- autorizaciones extraordinarias;
- checkpoint anterior, si lo hubo.

## 5. Pruebas y canarios

Los canarios de capacidad ejercitan la capacidad declarada. El eco básico demuestra sólo
transporte. Un recibo ausente conserva un conjunto vacío de capacidades; nunca significa
“todas”. Las evaluaciones locales se etiquetan con la ruta exacta y la versión del escenario.

La clase `probe/test` puede preferir rutas baratas, gratuitas o locales, pero sus resultados
no promueven una ruta a producción sin cumplir los requisitos de evidencia.

## 6. Aceptación

- Un fallo nunca dispara dos rutas en paralelo.
- Un `Retry-After` corto conserva ruta y checkpoint.
- Cuota, autenticación y saldo no provocan consultas de secretos/saldo.
- El relevo contiene sólo el checkpoint estructurado, no el historial.
- La elección final es determinista aun con empates completos.
- Cada recibo puede reproducir la política y evidencia utilizadas mediante sus hashes.
