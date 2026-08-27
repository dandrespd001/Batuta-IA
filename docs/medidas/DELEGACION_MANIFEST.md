# La primera delegación real

**2026-08-27.** Los cuerpos de `batuta-manifest` los escribió DeepSeek v4 Flash a través de
dsh. Este documento registra lo medido, porque es el ensayo a mano de lo que la Fase 3
automatiza, y porque casi todo lo que aprendimos salió de mirar la corrida, no de planearla.

Todo lo de aquí lo verificó el revisor ejecutando los comandos, no leyendo el informe del
modelo.

## Montaje

| pieza | valor |
|---|---|
| aislamiento | `git worktree` en la rama `encargo/batuta-manifest` |
| modelo pedido | `deepseek-official` / `deepseek-v4-flash` |
| contención | preset `batuta-escritura` = `workspace-write` + `approval: never` |
| entorno | `env -i` con `HOME`, `PATH`, `TERM`, `LANG` |
| ficheros de corrida | los dos que prescribe `providers/dsh.toml`, materializados a mano |
| contrato | 14 tests en rojo y firmas con `todo!()` |
| duración | ~15 minutos |

## Procedencia: lo pedido y lo ocurrido coincidieron

```
83 "provider":"deepseek-official"
83 "model":"deepseek-v4-flash"
```

**Las 83 peticiones en el modelo fijado.** Es la primera vez en toda la investigación de
dsh que lo pedido y lo ocurrido coinciden, y lo dice el registro de la máquina.

La contención también consta, anotada al crear la sesión:

```json
{"type":"permission/preset","seq":0,"data":{"preset":"batuta-escritura"}}
{"type":"sandbox/mode","seq":1,"data":{"mode":"workspace-write"}}
```

El recibo de la Fase 3 puede verificar **con qué jaula** corrió el encargo, no sólo qué
modelo. Eso no estaba en el plan: apareció al leer el registro.

## Herramientas: 95 llamadas, cero de web

```
bash 66 · read 16 · write 5 · edit 5 · todo_write 2 · str_replace_editor 1
```

Las herramientas web estaban **ofrecidas** —la composición de `headless` monta `web`,
`web-search-deepseek` y `tool-web`— y no se usaron. Ése es el argumento del §4 bis del
esquema: no se apagan, se observan. Un hecho medido vale más que una prohibición.

## Alcance: la allowlist se verifica sobre el diff, y ahora sabemos por qué

El encargo autorizaba cinco ficheros. **Durante** el trabajo el modelo se montó un proyecto
Rust aparte, `.scratch/spantest/` (65 MB), para comprobar la API de `toml::Spanned` antes de
usarla. Está **fuera** de la allowlist y **dentro** de lo que el sandbox permitía.

Al terminar lo había limpiado, y el diff final toca exactamente los cinco ficheros. Pero la
lección no depende de que limpiara:

> El sandbox de dsh confina al **worktree entero**; la allowlist de batuta es de cinco
> ficheros. dsh no conoce la allowlist y no puede hacerla cumplir. **Contención y alcance
> son cosas distintas**, y el alcance sólo se puede verificar sobre el diff resultante.

Consecuencia operativa: el diff se calcula **incluyendo lo no rastreado**. `git diff` a
secas no habría visto `.scratch/`.

## Lo que la delegación destapó del propio contrato

El modelo entregó 11 desviaciones numeradas. Tres cambiaron el diseño:

1. **`deny_unknown_fields` faltaba.** Un campo mal escrito se ignoraba en silencio, en un
   proyecto cuyo `TaskSpecDraft` lo lleva desde la Fase 1 precisamente para evitarlo.
2. **`schema_version` no soportada se reportaba como `Syntax`.** Decía una cosa por otra a
   quien leyera el mensaje.
3. **Una comprobación de la especificación era imposible.** El esquema pedía rechazar rutas
   «dentro del worktree» al cargar, y `parse()` es puro: el worktree no existe todavía. El
   modelo **paró y lo reportó** en vez de fingir que la implementaba. El error era del
   revisor.

Un modelo que se detiene ante un caso no previsto está haciendo lo correcto. Aquí destapó
un fallo de la especificación que dos lecturas humanas no habían visto.

## Trampas del transporte, medidas de paso

- **`npx` vuelca el argv completo en stderr.** 3.899 bytes de encargo produjeron 5.073 de
  stderr en líneas `npm notice`. Como R4 mete stderr íntegro en el recibo, con `npx` de por
  medio **el prompt acaba dentro del recibo**. Segunda razón, independiente de la primera,
  para que `program` apunte al enlace directo.
- **El registro no se puede leer mientras se escribe.** Un lector estricto ve un marco zstd
  completo con el JSONL partido y rechaza el fichero entero. Al terminar, íntegro:
  `zstd --test` pasa, 3.740.428 bytes, 4.526 líneas. El lector de batuta debe tolerar la
  cola partida —pero **una procedencia que no se puede leer es recibo en rojo**, nunca un
  hueco que se rellena con lo que se pidió.
- **Cada corrida deja dos sesiones**: la del encargo y la que genera el título.

## Lo que hizo que funcionara

Los tests eran el contrato, y fijaban los **mensajes**, no sólo los tipos: el que rechaza un
`parser` inválido exige que el mensaje liste los cuatro valores válidos. El modelo no tuvo
margen para un error pobre.

Y lo que salió mal —el `deny_unknown_fields`— es exactamente **lo único que no estaba en un
test**.
