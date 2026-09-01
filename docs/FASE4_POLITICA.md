# SPEC v2 — Política, evidencia investigada y routing

> **Archivo histórico desde 2026-08-31.** Las autoridades vigentes son
> [`specs/system/quality-research.md`](../specs/system/quality-research.md) y
> [`specs/system/state-policy-routing.md`](../specs/system/state-policy-routing.md). La paridad completa
> se registra en [`DOCUMENT_CLASSIFICATION.md`](DOCUMENT_CLASSIFICATION.md); el contenido siguiente se
> conserva íntegro.

**Estado: aprobado para implementación.** Este documento reemplaza la propuesta de Fase 4.

## 1. Vocabulario y límites

- Un **harness** es el ejecutor: DSH, Abacus, Codex, Claude, Qwen Code, Kimi Code u
  OpenCode.
- Una **ruta** identifica `harness + provider + model`. MiniMax y DeepSeek dentro de DSH
  son rutas de DSH, no harnesses.
- Un **alias** es un nombre humano que resuelve a una sola ruta. Dos rutas no pueden
  compartir el mismo alias.
- La **acción** describe el trabajo para el que se estima calidad. La calidad pertenece a
  `ruta + acción`; nunca al nombre del modelo aislado.
- Batuta no lee ni modifica credenciales, suscripciones o saldos. Autenticación y cuota se
  infieren exclusivamente del resultado que devuelve el harness.

## 2. Esquema de evidencia

`BenchmarkObservation` conserva el dato bruto: identificador, benchmark, versión,
escenario, configuración, ruta exacta si se conoce, revisión del modelo, métrica,
normalización `0..100`, URL, fecha, tipo de fuente y grupo de compatibilidad. Observaciones
de grupos, versiones o configuraciones incompatibles no se promedian.

`ActionProfile` contiene una cesta de componentes `{benchmark, scenario, weight}`. Los
pesos son enteros y suman exactamente 100.

`QualityProjection` es un resultado derivado e inmutable con:

- `researched_score`: media ponderada sólo sobre la cobertura utilizable;
- `effective_score`: override vigente o, si no existe, el investigado;
- `coverage`: suma de pesos cubiertos;
- `contributing_range`: mínimo y máximo de los resultados contribuyentes;
- procedencia, antigüedad, advertencias e identificador/hash de evidencia.

`ManualOverride` conserva puntaje manual, motivo no vacío, fecha, autor y el valor
investigado al que sustituyó. No edita ni elimina observaciones. Un override no convierte
evidencia ausente en verificada.

`SelectionMargin` es una tolerancia de elección `0..100`; no es incertidumbre estadística.

## 3. Proyección y confianza

1. Toda métrica se normaliza explícitamente a `0..100`.
2. La cobertura es la suma de pesos para los que hay evidencia compatible, vigente y
   aplicable a la ruta exacta.
3. Si varias observaciones compatibles cubren un componente, se usa la media de sus
   valores normalizados. No se mezclan grupos incompatibles.
4. `researched_score` se renormaliza sobre la cobertura disponible; cobertura cero no
   produce ningún puntaje.
5. La incertidumbre visible es rango, cobertura, edad y tipo de fuente. No se fabrica un
   intervalo estadístico.
6. Producción exige cobertura mínima, evidencia independiente reproducible o evaluación
   local de la ruta exacta, y antigüedad menor o igual al máximo del perfil.
7. Una fuente del fabricante puede contribuir, pero no verifica por sí sola una ruta.
8. `allow_unverified_quality` es una autorización separada, explícita y registrada.

El puente de descubrimiento DSH normaliza exclusivamente identidad, revisión,
capacidades, contexto y costes. Nunca conserva credenciales, saldo o suscripción.
Las rutas OpenCode se importan sólo si todos los componentes de coste existen,
son finitos y valen cero; el nombre `free` no constituye evidencia de coste.
`catalog import` sólo crea una propuesta sellada contra el hash activo;
`catalog apply` exige confirmación y vuelve a comprobar propuesta y base antes del
rename atómico.

Cestas iniciales editables:

| Acción | Componentes iniciales |
|---|---|
| `implementation`, `repair` | SWE-bench Verified + evaluación local exacta |
| `code_generation`, `code_execution` | escenarios LiveCodeBench |
| `tools` | BFCL |
| `web_research` | GAIA + BFCL Agentic |
| `review`, `documentation` | suite local específica |

## 4. Política y selección

`RouteRequestV2` no contiene candidatos, puntajes, proyecciones ni hashes. Esos
datos se ensamblan exclusivamente desde un `RoutingSnapshot` local validado. El
cliente tampoco puede suministrar el perfil de acción: Batuta resuelve los campos
omitidos contra el perfil persistido en la misma foto.

La política persistida usa `schema_version = 2`. La migración desde v1 es explícita: cada
modelo conserva `habilitado` y `esfuerzo`; los campos nuevos se reciben desde un documento
de migración o desde los valores declarados por el llamador, nunca mediante defaults
ocultos.

Para cada petición:

1. Resolver acción, calidad mínima y margen desde la petición o su perfil.
2. Descartar por capacidad, sensibilidad, evidencia, contexto, esfuerzo, cooldown y
   autorización, conservando todas las razones.
3. Exigir `effective_score >= minimum_quality`.
4. Calcular `Qmax` y conservar `score >= max(minimum_quality, Qmax - selection_margin)`.
5. Minimizar coste esperado: `predicted_tokens * relative_cost + handoff_penalty`, ajustado
   por la tasa reciente de éxito.
6. Desempatar por latencia p95 y finalmente por `RouteRef`, para determinismo.

Sólo se usan fallbacks aprobados. `allow_any_eligible` puede ser persistente o un override
de una petición. Las rutas `probe/test` nunca se promocionan por ese solo hecho.

## 5. Investigación bajo demanda

```
batuta research update [--all | --route <ruta> | --action <acción>]
batuta research status
batuta research apply <propuesta>
```

`update` usa un perfil web-capable aprobado y escribe una propuesta inmutable en staging.
Nunca modifica el conjunto activo. `apply` comprueba esquema, hash, fuentes, conflictos y
confirmación explícita antes de activar. La ruta investigadora no valida su propia calidad
sin otra fuente independiente o una evaluación local exacta. V1 admite evidencia curada;
no implementa scrapers particulares de leaderboards.
Una propuesta rechaza observaciones cuyo `RouteRef` sea la propia ruta investigadora;
esas mediciones deben llegar por otra ruta o por la suite local exacta.

## 6. Ejemplo JSON de petición y decisión

```json
{"schema_version":2,"request":{"schema_version":2,"action":"implementation","required_capabilities":["write"],"sensitivity":"internal","required_context":32000,"effort":"high","minimum_quality":78,"selection_margin":4,"predicted_tokens":32000,"allow_any_eligible":false,"allow_unverified_quality":false,"fallback":false,"class":"production","now":1787875200}}
```

```json
{"schema_version":2,"route":"dsh/deepseek-official/deepseek-v4-flash/2026-08","effective_score":82.5,"coverage":100,"evidence_hash":"sha256:…","policy_hash":"sha256:…","discarded":[]}
```

CLI, MCP y TUI deben serializar la misma estructura producida por la misma función pura.

## 7. Aceptación

- Una ruta tiene puntajes diferentes por acción y por harness.
- Cambiar pesos modifica la proyección sin mutar evidencia.
- Fuentes incompatibles o caducadas no se promedian.
- Una fuente sólo del fabricante no habilita producción.
- Un override conserva el valor investigado, razón, fecha y autor.
- Ninguna propuesta se activa sin `apply` confirmado.
- Toda decisión enumera descartes y conserva hashes de evidencia y política.
- MiniMax sigue siendo una ruta de DSH.
- No hay acceso de Batuta a credenciales, saldo o suscripción.
