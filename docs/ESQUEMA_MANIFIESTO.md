# Esquema del manifiesto de proveedor — Fase 2

> **Archivo histórico desde 2026-08-31.** La autoridad vigente es
> [`specs/system/manifests.md`](../specs/system/manifests.md). La paridad completa por sección y sus
> verificaciones están en [`DOCUMENT_CLASSIFICATION.md`](DOCUMENT_CLASSIFICATION.md). El contenido
> histórico que sigue se conserva íntegro y ya no debe editarse como contrato vigente.

**Estado: propuesta, pendiente de aprobación del Arquitecto.** Nada de
`batuta-manifest` se escribe antes.

Parte del esquema del brief §4 y le aplica lo que la Fase 0 midió. Cada campo nuevo
nombra la medición que lo obliga; ninguno está aquí «por si acaso» (R2).

---

## §1 El cambio de forma: un manifiesto puede materializar ficheros

El brief preveía un `patch_template`: una plantilla que batuta rellena por corrida para
fijar el modelo. La medición (`docs/medidas/DSH_HEADLESS.md` §10) enseñó que con **una**
plantilla no basta y que el problema no es «el parche del modelo» sino algo más general:

> Hay proveedores cuya configuración por invocación no cabe en `argv`, y viaja en
> ficheros que alguien tiene que escribir antes de arrancar el proceso.

dsh necesita **dos**: el parche de composición, que sólo redirige, y el documento de
settings, que es el que de verdad manda. Abacus no necesita ninguno.

Por eso el esquema no lleva un `patch_template` sino una lista genérica:

```toml
[[runtime_files]]
path   = "composicion.yml"   # relativo al directorio de corrida, nunca absoluto
format = "yaml"              # hoy sólo `yaml`: es el único demostrado (R2)

# Un documento que es una LISTA se declara con `entry` repetido:
[[runtime_files.entry]]
id = "settings"
[runtime_files.entry.config]
path = "{run_dir}/settings.yaml"
```

```toml
[[runtime_files]]
path   = "settings.yaml"
format = "yaml"

# Un documento que es un MAPA se declara con `content`:
[runtime_files.content.agent-default-model]
provider = "{route_provider}"
model    = "{route_model}"
```

Declarar `entry` y `content` a la vez, o ninguno de los dos, es **error de carga** que
nombra el fichero y el campo (R1). Son las dos únicas formas que un documento de
configuración toma, y distinguirlas explícitamente evita que batuta tenga que adivinar por
la forma de la tabla.

### Sustituciones derivadas del encargo

`{model}` y `{workdir}` salen del encargo directamente. Lo que no puede salir directo es un
valor del vocabulario de *otro*: dsh llama `workspace-write` a lo que batuta llama
`validated_patch`. Traducir eso dentro del núcleo sería meter el vocabulario de dsh en
`batuta-contract`. Así que la traducción la declara el manifiesto:

```toml
[substitutions.sandbox_mode]
read_only       = "read-only"
validated_patch = "workspace-write"
validated_apply = "workspace-write"
```

**Invariante que esto regala:** un mapa de sustitución tiene que cubrir el vocabulario
cerrado **entero**. Si mañana se añade un `write_mode` y un manifiesto no lo contempla, ese
manifiesto **falla al cargar** nombrando el valor que falta — en vez de elegir en silencio
o caer en un valor por defecto que nadie escribió. Es R1 y R8 trabajando juntas, y sale
gratis del hecho de que los vocabularios sean cerrados.

**Por qué genérico y no `patch_template`:** el núcleo no debe saber que existe una cosa
llamada «capa de parche de cordis». Sabe escribir ficheros que un manifiesto describe. La
tesis del proyecto —«añadir un proveedor es un fichero, nunca un parche»— se rompería en
la primera línea si el núcleo llevara dentro el vocabulario de dsh.

Reglas:

- `path` es **relativo al directorio de corrida** y se rechaza al cargar si es absoluto o
  si sube con `..`. Los ficheros de configuración de la corrida no son material del encargo
  y no deben aparecer en el `git diff`.
- La tercera comprobación —que no caiga **dentro del worktree**— **no puede hacerse aquí**,
  y decirlo importa más que tenerla: `parse()` es puro y el worktree no existe todavía
  cuando se carga un manifiesto. Pertenece a `batuta-exec`, que sí conoce el directorio de
  corrida y el worktree a la vez. Esta corrección salió de una delegación que se paró ante
  la contradicción en vez de fingir que la implementaba.
- Se materializan **antes** de arrancar el proceso y se borran al cerrar el recibo, salvo
  que la corrida falle: entonces se conservan, porque son parte de la evidencia.
- Sustituciones admitidas en `content` y en `argv`: `{model}`, `{route_model}`,
  `{workdir}`, `{run_dir}`, `{prompt}`, `{token}`. Cualquier otra llave es error de carga
  y se nombra (R1, R8).

---

## §2 El esquema completo

```toml
schema_version = 1
id             = "dsh"
kind           = "cli"

[executable]
program       = "~/.npm/_npx/1e7f6d9597241db0/node_modules/.bin/dsh"
version_pin   = "0.1.1-rc.2"
version_probe = ["-V"]
sha256        = "c0226687bb20f45c603ec6fe50f3de16d1c3510c3a803304ec575ef9bc366c62"
resolve       = ["~/.npm/_npx/1e7f6d9597241db0/node_modules/.bin/dsh"]

[auth]
method     = "oauth_cli"
store_path = "~/.dsh"

[invoke]
argv    = ["--profile", "headless", "--patch", "{run_dir}/composicion.yml", "{prompt}"]
workdir = "worktree"
prompt  = { via = "argv" }

[env]
allow = ["HOME", "PATH", "TERM", "LANG", "NODE_OPTIONS"]
deny  = ["DSH_TELEMETRY_MODE", "DSH_TELEMETRY_OTLP_URL"]

[response]
parser = "plain_text"

[[models]]
id              = "dsh-deepseek-v4-flash"
route_model     = "deepseek-v4-flash"
route_provider  = "deepseek-official"
roles           = ["implementation", "boilerplate", "tests", "bulk_refactor"]
max_sensitivity = "internal"

[canary]
prompt = "Responde exactamente con: {token}"
expect = "token_echo"

[provenance]
source = "session_log"
```

### Campos nuevos respecto del brief §4

| campo | qué resuelve | medición que lo obliga |
|---|---|---|
| `[executable] sha256` | R11 con un canal `rc` que se autoactualiza | §0: el binario vive en la caché de `npx`, bajo un directorio con hash |
| `[invoke] workdir` | el contrato de escritura entero | §4 y sonda 0.2: el `cwd` se respeta al leer y al escribir |
| `[[runtime_files]]` | fijar el modelo por corrida | §10: el `--patch` aterriza y pierde; manda el documento de settings |
| `[env] deny` | R5, allowlist explícita con excepciones nombradas | §9: `DSH_TELEMETRY_MODE` activa un exportador remoto |
| `[[models]] route_provider` | dsh separa ruta de modelo | `deepseek-official` ≠ `deepseek`: son dos rutas distintas al mismo proveedor |
| `[provenance] source` | que el recibo diga lo ocurrido, no lo pedido | §10: tres corridas fueron a un modelo que nadie pidió |

`prompt = { via = "argv" }` estrena la variante que la Fase 0 destapó, con su techo de
sensibilidad ya fijado en el contrato: `argv` no admite nada por encima de `internal`,
porque la línea de órdenes se lee desde `ps`.

---

## §3 Procedencia: el recibo anota lo observado

**El recibo no puede registrar el modelo que batuta pidió.** Esa es exactamente la
mentira que la sonda 0.2 destapó: se pidió `deepseek-v4-flash` tres veces y corrió
MiniMax las tres.

`[provenance] source = "session_log"` obliga a batuta a leer el `provider` y el `model`
que la máquina anotó, y a compararlos con lo pedido. Discrepancia = **recibo en rojo**,
no una nota al pie.

Advertencia que va en el propio manifiesto: `SESSION_FORMAT_VERSION` está clavado en `0`,
pre-release, sin compatibilidad prometida (`dsh-session/README.md:143`). Un esquema que
cambie es **canario en rojo**, nunca una excepción que se traga en silencio.

---

## §4 Nota de acoplamiento — obligatoria, y el brief la pedía

Hay tres clases de nombre en este manifiesto que **batuta no posee**:

1. `route_provider` y `route_model` — rutas configuradas *dentro* de dsh.
2. Los nombres de sección del documento de settings (`agent-default-model`, `llm-pi-ai`,
   `permission`…) y los de fila de composición (`settings`, `permission-presets`…) — los
   registran los plugins de dsh, y su README **no siempre coincide con su código**:
   `dsh-permission-presets` documenta `permissionPresets` y registra `permission`;
   `dsh-shell` documenta `bash` y registra `shell`. Manda el código.
3. `route_model` de abacus — el catálogo vive en el servidor: `ZAI GLM 5.3 Flash` **no
   aparece en ninguna parte del paquete instalado**, verificado por búsqueda.

**La divergencia se detecta con canario por modelo, no con validación estática**, y el
manifiesto debe decirlo para que nadie espere lo segundo. Un `route_model` que dsh no
conoce no es un error de carga: es un canario que falla, y así es como tiene que fallar.

### La bandera no es la autoridad — dos de dos

Medido en los dos proveedores que existen, con mecanismos distintos y el mismo resultado:

| proveedor | lo que parecía mandar | lo que mandaba de verdad |
|---|---|---|
| dsh | `--patch` sobre la composición | el documento de settings del usuario |
| abacus | `--model` en la línea de órdenes | la selección **dentro del producto** |

Dos de dos. No es una peculiaridad de dsh: es lo normal en un CLI de proveedor, donde la
bandera es una preferencia y la autoridad vive en el estado de la cuenta o del despliegue.

**Consecuencia, y es la que justifica `[provenance]`:** un manifiesto declara qué modelo
*quiere*, nunca qué modelo *habrá*. Sólo el recibo puede decir cuál corrió, y sólo si lo
leyó. Donde no se pueda leer —`source = "declared"`, que es el caso de abacus— el recibo
**no debe afirmar un modelo**: debe decir que no pudo comprobarlo.

Si el canario de abacus demuestra que su salida sí trae el modelo, esa ruta sube a
`session_log` y el manifiesto lo refleja. Antes no: se mide y luego se declara.

### Dónde vive de verdad la contención

Medido, porque es fácil equivocarse aquí y el error sería creer que hay una jaula donde no
la hay:

| palanca | dónde se declara | qué hace |
|---|---|---|
| preset de permisos | **composición** (`--patch`) | define `{ sandbox, approval }` |
| `permission.defaultPreset` | **settings** | *elige* uno de los presets definidos |
| `sandbox.mode` | composición | `read-only` \| `workspace-write` \| `danger-full-access` |
| sección `shell` | settings | timeouts y topes de salida. **No confina nada** |

`dsh-bash-local/README.md:42` es explícito: *«Unconfined by itself — this executor always
runs commands with the harness process's authority»*. Quien crea que poniendo algo en la
sección `shell` acota al modelo, no ha acotado nada.

Para un encargo de escritura, batuta declara en la composición un preset con
`sandbox = "workspace-write"` y `approval = "never"`, y lo selecciona desde el documento de
settings. `never` **no** es «acepta todo»: es rechazo determinista de lo que pida escalada
(`dsh-user-approval/lib/index.js:36`). Es exactamente lo que R5 exigía después de que
`--approval-mode auto` diera `auto_accept` 1 y 0 en dos canarios seguidos.

Corolario incómodo pero honesto: los nombres de sección de dsh son **API de facto** para
batuta. Si dsh renombra `permission`, batuta pierde la contención sin que nada falle al
cargar. El canario es la única red, y por eso no es opcional.

---

## §4 bis Herramientas del proveedor: no se apagan, se observan

**Decisión del Arquitecto:** las herramientas web quedan **siempre disponibles**. La
composición de `headless` monta `web`, `web-search-deepseek` y `tool-web`, y batuta no las
desactiva. No se complica el manifiesto con perillas de apagado.

Eso parece una renuncia y no lo es, porque cambia *prevenir* por **demostrar**, que es la
regla que ordena todo el proyecto:

> El registro de sesión anota cada `tool/call` con su nombre. El recibo puede decir
> exactamente qué herramientas se usaron, en vez de prometer cuáles estaban prohibidas.

En la delegación real de `batuta-manifest`, sobre 95 llamadas: `bash` 66, `read` 16,
`write` 5, `edit` 5, `todo_write` 2, `str_replace_editor` 1, y **cero** de web. La
herramienta estaba ofrecida y no se usó — y eso es un hecho medido, no una promesa.

**La regla que se sigue, y va al recibo:** un encargo cuyo `TaskSpec` no declara
`web_research` y cuyo registro muestra llamadas web es un **recibo en rojo**. No un aviso,
no una nota al pie. Es el mismo criterio que R2 aplica a las capacidades del modelo, ahora
del lado de lo que la corrida hizo de verdad.

Y tiene una ventaja sobre apagarlas: una perilla de apagado hay que mantenerla al día con
la composición de dsh, que batuta no posee. Un observador del registro sigue funcionando
aunque dsh añada mañana una herramienta que hoy no existe.

---

## §5 Lo que este esquema **no** hace

- **No valida el catálogo de modelos.** No puede: ver §4.
- **No desella credenciales.** dsh y abacus resuelven las suyas; `auth = "oauth_cli"` aquí
  significa «el CLI posee su almacén y batuta no aporta ningún secreto». Ningún proveedor
  usa hoy `sealed_credential`, y `batuta-cred` se queda en superficie mínima.
- **No fija el modelo por `argv`.** Se intentó, se midió, no funciona.
- **No implementa criptografía.** El `sha256` de `[executable]` lo calcula el crate `sha2`
  de RustCrypto. Hubo una versión escrita a mano —porque el árbol no traía crate de
  digest— y se sustituyó: cien líneas de FIPS 180-4 dentro de un cargador de manifiestos
  son una deuda que alguien tiene que auditar para siempre. Las pruebas de borde de bloque
  (`tests/sha256_bordes.rs`) se quedaron, y ahora vigilan al crate desde fuera.

---

## §6 Criterio de aceptación de la Fase 2 — **cumplido**

Binario, y cada punto tiene su prueba en `crates/batuta-manifest/tests/`. Estado al cierre
de la fase: **18 en verde**, los cuatro gates en código 0.

| # | Criterio | Prueba |
|---|---|---|
| 1 | Los **dos** proveedores se leen de fichero | `los_dos_manifiestos_del_repositorio_cargan` |
| 2 | Un valor fuera de vocabulario falla nombrando fichero, línea y valores válidos (R1, R8) | `un_valor_fuera_de_vocabulario_falla_nombrando_fichero_linea_y_validos` |
| 3 | Un `path` absoluto o con `..` falla **al cargar**, no al ejecutar | `una_ruta_absoluta_…`, `una_ruta_que_se_sale_…` |
| 4 | Una llave de sustitución desconocida lista las admitidas | `una_llave_desconocida_lista_todas_las_admitidas` |
| 5 | Un mapa de sustitución incompleto nombra la variante que falta | `un_mapa_de_sustitucion_incompleto_nombra_la_variante_que_falta` |
| 6 | `entry` y `content` juntos, o ninguno, falla al cargar | `declarar_lista_y_mapa_…`, `no_declarar_ni_lista_ni_mapa_…` |
| 7 | Un ejecutable irresoluble falla al cargar y dice dónde buscó (R1) | `un_ejecutable_que_no_existe_falla_al_cargar_y_dice_donde_buscó` |
| 8 | **Un campo desconocido falla al cargar y lo nombra** | `un_campo_desconocido_falla_al_cargar_y_lo_nombra` |
| 9 | **Una versión de esquema no soportada tiene su propio error** | `una_version_de_esquema_no_soportada_tiene_su_propio_error` |
| 10 | El `sha256` acierta en todos los bordes de bloque, y un hash falso se rechaza (R11) | `tests/sha256_bordes.rs` |

Los puntos **8 y 9 no estaban en la lista original**: salieron de revisar una delegación, y
los dos son doctrina del propio proyecto que el esquema incumplía. El 8 es el que más
importa —un manifiesto no puede ser más laxo que un encargo, y `TaskSpecDraft` lleva
`deny_unknown_fields` desde la Fase 1—: sin él, escribir `version_pinn` en vez de
`version_pin` dejaba el pin sin efecto y R11 sin red, en silencio.

El punto 10 tampoco estaba, y guarda algo que la lista original no anticipaba: que el hash
lo calcule un crate o cien líneas propias es indiferente para la prueba, porque comprueba
desde fuera. Se escribió contra una implementación artesanal y sobrevivió intacta a
sustituirla por `sha2`.

Los dos manifiestos —`providers/dsh.toml` y `providers/abacus.toml`— son el banco de
pruebas de esta lista. Que abacus necesite **cero** `[[runtime_files]]` mientras dsh
necesita dos es la comprobación de que el campo es genérico y no dsh con otro nombre.

---

## §7 Integración con dsh: una delegación, un hilo navegable

Objetivo del Arquitecto: que cada delegación quede en **un solo chat**, y que desde
`dsh web` se vea qué se hizo y su trayectoria. Lo que sigue está medido sobre dsh
`0.1.1-rc.2`, y una de las tres respuestas es un «no» que conviene decir antes de diseñar
sobre una ilusión.

### 1. La visibilidad ya funciona, y no hace falta nada

La interfaz descubre las sesiones **escaneando el directorio de persistencia**, no el
registro de workspaces: *«the registry calls `SessionPersistence.list()`»*
(`dsh-workspace/README.md:21`). Una delegación headless **ya aparece** en `dsh web` con su
trayectoria completa.

Lo que aporta `workspace.json` es sólo **agrupación**: una sesión cuya ruta no está
registrada como workspace aparece sin grupo. batuta no necesita escribir ahí —y no debería:
ese fichero lo posee `dsh-workspace`.

### 2. «Un solo chat» no es alcanzable, y por qué

`dsh-headless/lib/index.js:71` crea la sesión así:

```javascript
sessionId: SessionId(`session-${randomUUID()}`),
```

**Cada corrida es una sesión nueva.** No hay bandera, ni campo de configuración de la fila
`headless-runner`, ni servicio expuesto para continuar una existente. Lo más parecido es
`ctx.sessions.fork()`, que registra linaje y hereda `delegationDepth`
(`dsh-session/README.md:17`), pero **headless no lo expone**.

Diseñar suponiendo lo contrario habría costado un sprint. Se dice aquí para que no se
intente.

### 3. Lo que sí da el efecto que se busca: el worktree es la identidad

`projectKey(cwd)` (`dsh-session-persistence-jsonl/lib/index.js:106`) es **determinista**:
el mismo `cwd` produce siempre el mismo directorio de proyecto. De ahí sale una regla de
batuta que antes era una comodidad y ahora es un requisito:

> **El worktree de un encargo es estable durante todo el encargo, reintentos incluidos.**

Con eso, un encargo = un worktree = **un grupo en la interfaz**, con una sesión por intento,
juntas y en orden. Y son a lo sumo tres, porque `MAX_REPAIRS = 2`: dos fallos seguidos
reencaminan el trabajo en vez de repararlo otra vez. Lo que se pierde frente a «un solo
chat» es un tabique entre intentos, no la trazabilidad.

Aviso de la propia documentación: la normalización del `cwd` es *«intentionally lossy»*
(`dsh-session-persistence-jsonl/README.md:19`), así que dos rutas muy largas pueden
colisionar en el mismo directorio. Los worktrees de batuta deben tener rutas cortas y
distinguibles cerca del final.

### 4. El título de la interfaz sale del encargo

`dsh-session-title-first-prompt-llm` *«summarizes the first eligible human message through
`ctx.llm`»*. El título que el Arquitecto verá en la lista **lo decide la primera línea del
encargo**, resumida por el modelo. Es la única palanca que hay sobre esa etiqueta, y es
gratis:

> Todo encargo empieza por una línea identificadora:
> `ENCARGO <id> · <crate o área> · intento <n>/<max>`

Es un resumen, no una copia literal, así que el título será *reconocible*, no exacto.

Coste que conviene saber: **cada sesión gasta una llamada de modelo extra** para titularse,
y por defecto hereda la ruta del encargo. Se puede enrutar aparte fijando `provider` y
`model` en la fila del titulador, si algún día el gasto importa.

### 5. El índice es el recibo, no dsh

`SessionHeader` es inmutable y no admite metadatos propios: sus campos son
`version`, `id`, `createdAt`, `cwd?`, `parentSession?`, `seedLength?`, `origin?` y
`delegationDepth?` (`dsh-session-persistence/README.md:63`). `parentSession` y `seedLength`
sólo los escribe `fork()`, que headless no expone.

Conclusión, y es una decisión de diseño, no una carencia: **batuta no intenta enlazar los
intentos dentro de dsh.** El recibo lleva, en orden, el id de sesión de cada intento y su
veredicto. dsh enseña cada hilo; el recibo dice qué hilo fue qué. Es la misma división que
en la procedencia: dsh registra el hecho, batuta lo lee y lo interpreta.

### Lo que esto añade al manifiesto

Nada. Ni un campo. Las tres consecuencias son reglas de `batuta-exec` —worktree estable,
primera línea identificadora, ids de sesión en el recibo— y por eso viven en este documento
y no en `providers/dsh.toml`. Un manifiesto que creciera para esto estaría describiendo a
batuta en vez de al proveedor.
