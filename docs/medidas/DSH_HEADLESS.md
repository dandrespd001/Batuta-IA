# Medición de `dsh --profile headless`

Fase 0.2 del plan revisado. **Medido el 2026-08-27**, en la máquina del Arquitecto.
Todo lo de aquí sale de corridas reales, no de documentación.

Sin esto, la Fase 3 se escribe a ciegas.

## 0. Qué binario es

| dato | valor |
|---|---|
| versión | `0.1.1-rc.2` |
| paquete | `@deepseek-ai/dsh` |
| ruta real | `~/.npm/_npx/1e7f6d9597241db0/node_modules/@deepseek-ai/dsh/lib/bin.js` |
| sha256 del bin | `c0226687bb20f45c603ec6fe50f3de16d1c3510c3a803304ec575ef9bc366c62` |
| node | `v26.7.0` |
| **¿en `PATH`?** | **no** |

**No está en `PATH`.** Vive en la caché de `npx`, bajo un directorio con hash
(`1e7f6d9597241db0`). Un `npx @deepseek-ai/dsh@latest` puede reescribirlo sin avisar:
es exactamente la trampa de R11 —«un CLI en canal `latest` se autoactualiza y rompe el
transporte en silencio»— y encima es una `rc`.

Consecuencia para el manifiesto: `resolve` no puede ser `$PATH`, y `version_pin` sin
hash no basta.

## 1. Formato exacto de la salida

**Texto plano, sin envoltorio.** No hay JSON, ni banner, ni marca de fin. Es el mensaje
final del asistente, literal.

```
$ dsh --profile headless "Responde exactamente con: BATUTA-CANARY-7F3A9"
```
```
stdout (21 B): \n B A T U T A - C A N A R Y - 7 F 3 A 9 \n
```

El salto inicial **es del modelo, no de dsh**: una segunda corrida con otro token
devolvió 18 B sin salto delantero. Es decir, dsh no añade ni quita nada, y batuta no
puede recortar bordes a ciegas.

`parser = "plain_text"` basta. R14 se cumple sin plugin en esta ruta.

## 2. Códigos de salida

| caso | exit | stdout | stderr |
|---|---|---|---|
| éxito | `0` | el mensaje | 0 B |
| sin tarea | `1` | 0 B | 79 B, mensaje limpio |
| perfil inexistente | `1` | 0 B | 1098 B, **traza de Node cruda** |

Los dos fallos dan **el mismo `0` bytes en stdout** y el mismo `exit 1`. Lo único que
los distingue es stderr, y uno de ellos es un stack trace de `dsh-app-boot/lib/index.js:543`.

Es literalmente el fallo de R4: tres causas distintas con el mismo 0-bytes, y el error
literal en stderr las tres veces. **El recibo tiene que llevar stderr íntegro o no
sirve.**

## 3. ¿stderr separado de stdout?

**Sí, limpiamente.** En éxito stderr queda en 0 B; en fallo stdout queda en 0 B. No hay
mezcla en ninguno de los cuatro casos medidos.

## 4. ¿respeta un `cwd` distinto?

**Sí, y además lo usa como identidad de sesión.** Ejecutado desde
`…/scratchpad/cwd_prueba` creó:

```
~/.dsh/sessions/--tmp-claude-1000-…-scratchpad-cwd_prueba--/session-<uuid>/session.jsonl.zstd
```

y el registro guarda `"cwd": "…/cwd_prueba"`. El prompt del sistema que recibe el modelo
también lleva el cwd dentro.

Para la Fase 3 esto es exactamente lo que hace falta: `cwd` = worktree, y batuta calcula
`git diff` ella misma.

Efecto secundario a tener en cuenta: **cada worktree deja una sesión nueva en disco**.

## 5. El prompt entra **sólo por argv**

```
$ echo "…" | dsh --profile headless
error: a task is required, for example: dsh --profile headless "run the tests"   (exit 1)
```

No hay `stdin`, no hay `--file`. La tarea son los argumentos posicionales, unidos por
espacios.

**Esto rompe un supuesto de la Fase 1**: el vocabulario `prompt_delivery` sólo tiene
`stdin` y `file`, y ambos se eligieron porque argv es visible en `ps`. Hace falta una
variante `argv`, y con ella una regla de sensibilidad: un encargo con `sensitivity`
por encima de `internal` no debería viajar por argv.

## 6. La trampa gorda: no hay selección de modelo por invocación

Tres hechos medidos, en orden:

1. `--dump-config` del perfil `headless` dice
   `agent-default-model: {provider: deepseek-official, model: deepseek-v4-flash}`.
2. `~/.dsh/settings.yaml` dice `{provider: minimax, model: MiniMax-M2.7}`.
3. **La corrida real fue a `minimax / MiniMax-M2.7`.** Consta en el registro de sesión:
   `{"type":"request/context","data":{"provider":"minimax","model":"MiniMax-M2.7","contextWindow":204800}}`

`--dump-config` y `--dump-default-config` devuelven **exactamente lo mismo** (diff vacío):
el fichero de settings del usuario no está en el árbol; lo aplica en ejecución el plugin
`dsh-settings-file`, después.

Y lo que remata: se probó `--patch` con un overlay que fijaba
`deepseek-official / deepseek-v4-flash`. **El árbol cambió** (se ve en `--dump-config`)
y **la corrida siguió yendo a MiniMax-M2.7**.

> `--dump-config` no es autoritativo sobre qué modelo corre. Es el R3 de dsh: la
> inspección enseña un reflejo, no el hecho.

**Consecuencia que se creyó directa —y era falsa. Ver §10.** Se concluyó aquí que el hito
de la Fase 3 «no es alcanzable por invocación». Lo es: el `--patch` puede redirigir el
**`path` de la fila `settings`** a un documento que batuta materializa por corrida, y
entonces el selector deja de ser estado global del usuario. Medido y probado en §10.

Lo que sí queda en pie de este apartado, y es lo que importa: **`--dump-config` no es
autoritativo sobre qué modelo corre.** El diagnóstico era correcto; el pronóstico no.

## 7. La salida: `DSH_HOME` sí se honra

```
$ DSH_HOME=<dir vacío> dsh --profile headless --dump-config     # exit 0
```

Creó `<dir>/profiles/headless` y `<dir>/profiles/node_modules` y compuso el mismo árbol.

Eso da a batuta un `DSH_HOME` propio por delegación, con su `settings.yaml`, sin tocar
el del Arquitecto — que es la forma correcta de fijar el modelo.

**Pregunta que quedó abierta y hoy está cerrada; ver §10.** Se dejó así: un `DSH_HOME` aislado no hereda `~/.dsh/.credentials.yaml`. Pero
`settings.yaml` declara los proveedores por **variable de entorno**
(`apiKeyEnv: MINIMAX_API_KEY`, `KIMI_CODING_API_KEY`, `OPENCODE_API_KEY`), así que es
plausible que baste con pasar la variable. Hay que comprobarlo con una corrida real
antes de escribir `providers/dsh.toml`.

Si bastan las variables, `auth = "oauth_cli"` **no** describe a dsh y el manifiesto
necesita `sealed_credential` con su `env`. Eso mueve `batuta-cred` de vuelta a la ruta
principal, al revés de lo que supone el plan.

## 8. Procedencia: recuperable, pero fuera de banda

El registro de sesión (`session.jsonl.zstd`, zstd + JSONL con eventos tipados) lleva
`provider`, `model`, `contextWindow`, `cwd` y `delegationDepth`. stdout **no lleva nada
de eso**.

Batuta puede leerlo para el recibo, pero es un fichero fuera del worktree, indexado por
cwd, comprimido. Alternativa más barata y honesta: batuta registra en el recibo el modelo
que **fijó** y verifica contra el registro sólo cuando importe.

## 9. Telemetría

El árbol trae `session-telemetry-otel` con
`mode: process.env.DSH_TELEMETRY_MODE || 'DISABLED'` y exportador a
`harness-telemetry.deepseeksvc.com`. Por defecto **desactivado**.

Para la allowlist de entorno de R5: no propagar `DSH_TELEMETRY_MODE` ni
`DSH_TELEMETRY_OTLP_URL` salvo decisión explícita.

## Resumen para los manifiestos

| campo | valor medido |
|---|---|
| `program` | `node <ruta npx>/lib/bin.js` — no hay binario en `PATH` |
| `version_pin` | `0.1.1-rc.2` + sha256 (canal `rc`, se mueve) |
| `argv` | `["--profile", "headless", "{prompt}"]` |
| `prompt.via` | **`argv`** — variante que hoy no existe en el contrato |
| `response.parser` | `plain_text` |
| `workdir` | honrado; además identifica la sesión |
| selección de modelo | **no por invocación**; vía `DSH_HOME` propio + `settings.yaml` |
| `auth` | por comprobar: `apiKeyEnv` sugiere `sealed_credential`, no `oauth_cli` |


---

## 10. Sonda 0.2b: el modelo **sí** se fija por corrida

Medido el **2026-08-27**, después de §6. Corrige su pronóstico y cierra la pregunta de §7.

### La cadena de precedencia, documentada

`dsh-settings/README.md:5` — la resolución de cada sección es *schema defaults* → *composition
`base`* → **sección del documento de usuario**. El `--patch` toca la segunda capa; el
documento de settings es la tercera y gana. Por eso el parche aterrizaba y perdía.

No hay bandera ni variable de entorno para el modelo: `--model`, `--provider`, `DSH_MODEL`
y `DSH_PROVIDER` buscados literalmente en los 198 paquetes, cero coincidencias.

### El mecanismo que sí funciona

Redirigir la fila `settings` con el mismo `--patch`:

```yaml
- id: agent-default-model
  config: { provider: deepseek-official, model: deepseek-v4-flash }
- id: settings
  config: { path: <dir-de-corrida>/settings_corrida.yaml }
```

y materializar ese documento con la sección `agent-default-model`.

### La medida, diferencial y observacional

| tiro | parche | exit | stdout | registro de sesión |
|---|---|---|---|---|
| PIN | sí | `0` | `PONG` | `deepseek-official` / `deepseek-v4-flash` |
| CONTROL | no | `1` | 0 B | `llamacpp` / `Tiel-Coder-35B-A3B-GGUF` |

El veredicto sale del registro de sesión, no de `--dump-config` ni de lo que se pidió.

**Y el CONTROL dio el hallazgo gordo:** debía decir `minimax`, dijo `llamacpp`, y al releer
`~/.dsh/settings.yaml` decía ya `opencode / nemotron-3-ultra-free`. Tres modelos distintos
en media hora, uno de ellos apuntando a un servidor local caído.

> El documento de settings del anfitrión es **estado vivo, editado a mano mientras batuta
> corre**. Heredarlo no sería sólo inseguro: haría la delegación **no reproducible**, con
> dos corridas del mismo encargo en dos modelos distintos y un recibo incapaz de notarlo.

Ése, y no el argumento de diseño, es el motivo por el que batuta posee su documento.

### Lo que arrastra poseer el documento

Redirigir `path` mueve **las doce secciones a la vez**, no sólo la del modelo. Las que
importan en headless: `agent-default-model`, `llm-pi-ai` (rutas de proveedor),
`llm-deepseek`, `agent-loop`, `agent-presets`, `permission` y `shell`. Las cuatro restantes
(`locale`, `ui-theme`, `ui-conversation`, `ui-onboarding`) son de interfaz.

Que `permission` entre en el lote **no es un peaje: es la contención de R5** pasando a
manos de batuta en vez de heredarse del anfitrión. Con un matiz medido que conviene no
confundir:

- La sección `permission` del documento de settings tiene **un solo campo**:
  `{ defaultPreset: <nombre de preset> }` (`dsh-permission-presets/lib/index.js:134`).
  *Elige* un preset; no lo define.
- Los presets —y con ellos el modo de sandbox y la política de aprobación— son
  **configuración de composición**, no de settings. Van en el fichero de `--patch`.
- La sección `shell` **no restringe nada**: son timeouts y topes de salida. El ejecutor lo
  dice de sí mismo: *«Unconfined by itself — this executor always runs commands with the
  harness process's authority»* (`dsh-bash-local/README.md:42`). La contención no está
  ahí.

Modos de sandbox, de menos a más permisivo: `read-only`, `workspace-write`,
`danger-full-access`, y *«Default `read-only` (fail-safe)»*
(`dsh-sandbox-policy/README.md:13`).

Y un hallazgo que vale por sí solo: la política de aprobación admite `never`, que **no**
significa «acepta todo» sino **rechazo determinista** de cualquier operación que pida
escalada (*«`'never'` rejects before interactive dispatch»*, `dsh-user-approval/lib/index.js:36`).
Es literalmente lo que R5 pedía después de que `--approval-mode auto` diera `auto_accept`
1 y 0 en dos canarios seguidos para la misma llamada: contención determinista, sin
clasificador.

### Credenciales: la pregunta de §7, cerrada

Orden de `dsh-credentials-local/README.md:9-12`, ganador primero:

1. **entorno heredado** — siempre gana
2. `$DSH_HOME/.credentials.yaml`
3. `<cwd de la invocación>/.env`
4. `$DSH_HOME/.env`

Y `dsh-llm-deepseek/README.md:80`: *«Configuration carries only `apiKeyEnv`, never a literal
key»*. Es R10 ya implementado por dsh.

Como el diseño elegido **conserva el `DSH_HOME` del Arquitecto** (sólo se redirige el
documento de settings), la corrida PIN resolvió la credencial de `deepseek-official` sola y
respondió. `auth = "oauth_cli"` sigue describiendo a dsh: batuta no aporta ningún secreto.
`batuta-cred` se queda fuera de la ruta principal, como suponía el plan.

### Dos trampas nuevas para el manifiesto

1. **`npx` ensucia stderr.** Cada invocación añade líneas `npm notice`. El invariante
   documentado *«successful runs keep stderr empty»* sólo se cumple invocando el binario
   directo: hay un enlace en `node_modules/.bin/dsh` → `@deepseek-ai/dsh/lib/bin.js`. Como
   R4 mete stderr íntegro en el recibo, un canario con `npx` de por medio no lo vería vacío
   nunca. **`program` apunta al enlace, no a `npx`.**
2. **Cada corrida deja una sesión en disco.** Aquí se afirmó lo contrario —«dos: la del
   encargo y la del título»— y era falso: el generador de título trabaja **dentro** de la
   misma sesión, y por eso aparece en el registro como un `provider` distinto pero no como
   un directorio aparte. Comprobado en dos delegaciones reales, una sesión cada una.

   Lo que sí se sostiene es la consecuencia práctica: **identificar la sesión de una corrida
   por instantánea antes/después**, y no por «la más reciente». Un directorio de proyecto
   acumula las sesiones de todos los intentos, y hubo un caso con dos sesiones bajo el mismo
   `cwd` que no supe explicar. La instantánea es inmune a eso; «la más reciente» no.

### Contradicciones entre README y código, encontradas de paso

| paquete | README dice | el código registra |
|---|---|---|
| `dsh-permission-presets` | `permissionPresets` | `permission` (`lib/index.js:25`) |
| `dsh-shell` | `bash` | `shell` (`lib/index.js:64`) |

Manda el código.
