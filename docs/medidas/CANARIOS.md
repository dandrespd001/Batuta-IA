# Los canarios reales

Cuatro corridas contra proveedores de verdad, el 28 de agosto de 2026. Los recibos
están en `docs/medidas/recibos/`, tal como los escribió `batuta`, sin editar.

Un canario es la corrida más pequeña que demuestra que un proveedor responde. Se genera
un token irrepetible, se pide que lo devuelva, y se comprueba que **volvió ése**. Nunca se
busca una subcadena en un juicio que el propio sistema emitió: el fallo que lo paga
devolvía `QUOTA_UNAVAILABLE` en 126 ms sin tocar la red, porque leía el `status` del mismo
fichero que él debía informar (R3).

---

## 1 · dsh, verde y con el modelo confirmado

```
$ DSH_HOME=~/.dsh BATUTA_PROVIDERS=providers \
  batuta canary --provider dsh --model dsh-deepseek-v4-flash
recibo: ~/.local/state/batuta/recibos/dsh-dsh-deepseek-v4-flash-1787924660056.json
verde — modelo confirmado
EXIT=0
```

| Campo | Valor |
|---|---|
| `exit_code` | `0` |
| `duration_ms` | `2581` |
| `stdout` | el token, exacto y solo |
| `stderr` | **vacío** — el invariante de headless, medido |
| `observed.provider` / `.model` | `deepseek-official` / `deepseek-v4-flash` |
| `observed.sandbox_mode` | `read-only` |
| `observed.permission_preset` | `batuta-lectura` |
| `observed.tool_calls` | `[]` |
| `model_confirmed` | `true` |

**Lo que demuestra.** El recibo no dice que el modelo fuera el pedido: lo **contrasta**
contra lo que la máquina anotó en su registro de sesión. Y la contención que el manifiesto
pidió aparece en el registro del propio proveedor, no en la palabra de batuta: un canario
es `read_only`, y el sandbox quedó en `read-only` con el preset `batuta-lectura`.

Recibo: [`recibos/dsh-verde.json`](recibos/dsh-verde.json).

---

## 2 · dsh discrepante: **exit 0, token correcto, y rojo igual**

Es el criterio que más importa de la fase y el que no estaba en el brief: la única forma de
demostrar que el recibo no miente.

`pruebas/discrepante/dsh.toml` es idéntico a `providers/dsh.toml` salvo en una cosa: su
documento de settings fija `deepseek-official / deepseek-v4-flash` con literales, mientras
su `[[models]]` declara pedir `deepseek-v4-flash-que-nadie-corrio`.

```
$ BATUTA_PROVIDERS=pruebas/discrepante batuta canary --provider dsh --model dsh-deepseek-v4-flash
rojo — Red(ProvenanceMismatch {
    requested: "deepseek-v4-flash-que-nadie-corrio",
    observed:  "deepseek-v4-flash" })
EXIT=1
```

| Campo | Valor |
|---|---|
| `exit_code` | **`0`** |
| `stdout` | el token, exacto |
| `stderr` | vacío |
| `duration_ms` | `2605` |
| `verdict` | **rojo**, `ProvenanceMismatch` |
| `model_confirmed` | `false` |

**Todo lo que una comprobación ingenua llamaría éxito está ahí**: el proceso salió con 0,
devolvió exactamente el token que se le pidió, y no escribió una línea en stderr. El recibo
sale rojo porque el registro nombra un modelo distinto del pedido. Es la forma exacta del
fallo que originó el proyecto —se pidió un modelo, corrió otro, y todo salió con código
cero— y ahora es imposible de tragarse.

Recibo: [`recibos/dsh-discrepante-rojo.json`](recibos/dsh-discrepante-rojo.json).

### El primer intento midió otra cosa, y también cuenta

La primera versión del manifiesto discrepante fijaba `minimax / MiniMax-M2.7`. El canario
salió rojo, pero por `ProcessFailed { exit_code: 1 }`: `dsh: NO_ADAPTER: no adapter
registered for provider "minimax"`. Rojo por el motivo equivocado.

Sirvió para comprobar dos cosas que no se estaban buscando. El **orden** del veredicto
aguantó un caso real: primero lo que impidió ejecutar, y sólo después lo que se ve leyendo
el registro. Un recibo que hubiera dicho «modelo equivocado» cuando el proceso ni arrancó
habría diagnosticado mal. Y el campo `observed` registró la verdad de todos modos
—`minimax / MiniMax-M2.7`—, así que no se perdió nada por no ser el veredicto.

Recibo: [`recibos/dsh-discrepante-sin-adaptador-rojo.json`](recibos/dsh-discrepante-sin-adaptador-rojo.json).

---

## 3 · abacus, verde **sin confirmar el modelo**

```
$ BATUTA_PROVIDERS=providers batuta canary --provider abacus
verde — modelo sin confirmar: el proveedor no deja registro legible
EXIT=0
```

| Campo | Valor |
|---|---|
| `exit_code` | `0` |
| `duration_ms` | `16405` |
| `stdout` | el token, exacto |
| `route_model` | `ZAI GLM 5.3 Flash` |
| `runtime_files` | `[]` — abacus no necesita ninguno |
| `observed` | `null` |
| `model_confirmed` | **`false`** |

**Lo que demuestra, y es el motivo de que exista el tercer estado de la procedencia.** Un
verde de abacus significa *«el transporte funciona»*, no *«corrió ZAI GLM»*. Las dos cosas
a la vez, y dichas: sin el tercer estado sólo cabían dos salidas y las dos eran malas
—rojo siempre, o un `Ok` fabricado con el modelo pedido, que es exactamente la mentira que
el proyecto existe para impedir—.

**Y ni una línea del núcleo sabe qué es Abacus.** Corre porque su manifiesto lo describe.
Ésa es la tesis del proyecto, medida: *añadir un proveedor es un fichero, nunca un parche*.

Recibo: [`recibos/abacus-verde.json`](recibos/abacus-verde.json).

---

## 4 · El rojo que enseñó algo: `USER`

El primer canario de abacus salió rojo en **443 ms**:

```
Something went wrong — Abacus.AI CLI ran into an unexpected error.
  Unsupported state or unable to authenticate data
```

Y `abacusai auth status` con el entorno completo decía que la sesión era válida. La CLI
deriva de `$USER` la clave con que descifra su almacén de credenciales, así que bajo
`env -i` sin `USER` no puede leer sus propias credenciales.

Se bisecaron ocho variables: `USER`, `LOGNAME`, `SHELL`, `XDG_RUNTIME_DIR`,
`DBUS_SESSION_BUS_ADDRESS`, `XDG_SESSION_TYPE`, `DISPLAY`. **Sólo `USER` lo arregla.**

R5 no dice «pasa poco»: dice que nada se hereda sin nombrarlo. Una variable que hace falta
y está nombrada cumple la regla; una que hace falta y se adivina, no. `USER` está ahora en
el `allow` de `providers/abacus.toml` con la medición escrita al lado.

Recibo: [`recibos/abacus-sin-user-rojo.json`](recibos/abacus-sin-user-rojo.json).

---

## El canario del eco, y por qué va primero

Antes de cada canario real corre el del `eco` (`/bin/echo`), que ejercita la cadena entera
—sustitución, materialización, entorno, proceso, captura y sellado— **sin una sola llamada
de red**. Su valor no es la cobertura: es la **atribución**. Si el eco está verde y el real
falla, el fallo es del proveedor. Sin esa comprobación previa, cada fallo obliga a adivinar
de qué lado está, que es como se pierden las tardes.

Los cuatro rojos y verdes de arriba se leyeron así, y por eso cada uno pudo atribuirse en
un minuto en vez de en una tarde.

---

## Lo que estos canarios dejan pendiente

**El stderr de abacus lleva `model: ZAI_GLM_5_3_FLASH`.** Su procedencia *sí* es legible:
en otro sitio y en otro formato que la de dsh. Un tercer valor de `provenance.source`
—leer del stderr con un patrón declarado en el manifiesto— la haría comprobable, y el
`model_confirmed: false` de arriba pasaría a `true` sin tocar el núcleo. Es trabajo de la
Fase 4, y es la clase de cambio que este diseño tenía que permitir.

**`RedReason::DigestMismatch` y `::ExecutableUnresolved` no tienen productor.** Su
condición se detecta **al cargar** el manifiesto (R1), antes de que exista corrida de la
que sellar recibo. O se les da un recibo de «corrida que no llegó a empezar», o sobran. No
se decide aquí porque la decisión afecta a lo que la Fase 4 haga con los recibos.
