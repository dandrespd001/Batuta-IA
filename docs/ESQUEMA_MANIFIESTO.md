# Esquema del manifiesto de proveedor — Fase 2

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

- `path` es **relativo al directorio de corrida** y se rechaza si es absoluto, si sube con
  `..` o si cae dentro del worktree. Los ficheros de configuración de la corrida no son
  material del encargo y no deben aparecer en el `git diff`.
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

## §5 Lo que este esquema **no** hace

- **No valida el catálogo de modelos.** No puede: ver §4.
- **No desella credenciales.** dsh y abacus resuelven las suyas; `auth = "oauth_cli"` aquí
  significa «el CLI posee su almacén y batuta no aporta ningún secreto». Ningún proveedor
  usa hoy `sealed_credential`, y `batuta-cred` se queda en superficie mínima.
- **No fija el modelo por `argv`.** Se intentó, se midió, no funciona.

---

## §6 Criterio de aceptación de la Fase 2

Binario, y sale del brief §7:

1. `batuta providers list` enumera **dos** proveedores leídos de fichero.
2. Un manifiesto roto **falla al cargar nombrando el campo y la línea** (R1).
3. Un `[[runtime_files]]` con `path` absoluto, con `..`, o que caiga dentro del worktree,
   falla al cargar (no al ejecutar).
4. Una llave de sustitución desconocida falla al cargar listando las admitidas (R8).
5. Un `[substitutions.<clave>]` que no cubra todas las variantes de su vocabulario falla al
   cargar nombrando la que falta.
6. Declarar `entry` y `content` en el mismo `[[runtime_files]]`, o ninguno, falla al cargar.
7. `bash scripts_ci/local_gates.sh` en código 0.

Los dos manifiestos ya escritos —`providers/dsh.toml` y `providers/abacus.toml`— son el
banco de pruebas de esta lista. Que abacus necesite **cero** `[[runtime_files]]` mientras
dsh necesita dos es la comprobación de que el campo es genérico y no dsh con otro nombre.
