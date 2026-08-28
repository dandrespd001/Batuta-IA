# Fase 3 — ejecución, recibos y worktree

**Estado: propuesta, pendiente de aprobación.** Nada se escribe antes.

El brief planeó esta fase antes de medir el transporte. Lo medido cambia el alcance en tres
sitios y lo confirma en el resto; cada cambio dice de dónde sale.

---

## §1 Alcance: tres crates

`batuta-exec`, `batuta-receipt` y `batuta-lease`.

**Sobre el lease hubo un error de alcance en el primer borrador de este documento, y lo
corrigen los propios requisitos.** Se propuso aplazarlo con el argumento de que hoy no hay
delegaciones concurrentes. Pero R6 no dice «matar la tarea mata el árbol»: dice **«matar la
tarea mata el árbol *y libera el lease*»**, y su verificación en el brief §7.4 es literal —
*«matar la tarea deja cero hijos y cero leases»*. Sin leases, **R6 no se puede comprobar**,
que es exactamente lo que esta fase existe para demostrar. El fallo que la paga tampoco era
hipotético: `TaskStop` dejaba el hijo vivo gastando cuota **y su lease de repositorio
bloqueando a cualquier otro modelo** con `AdmissionUnavailable`.

La lección, que vale más que la corrección: aplacé una pieza mirando su utilidad presente en
vez de mirar la regla que la exige. «Hoy no hace falta» y «hoy no se puede verificar la regla
sin ella» son cosas distintas.

`batuta-plugin` **sí** se queda para después: `parser = "plain_text"` basta para los dos
proveedores que existen, medido, y la ABI C espera a que haya un transporte que la necesite.
Ahí el argumento sí se sostiene, porque ninguna regla queda sin comprobar por no tenerlo.

**El hito se parte en dos, y el primero no necesita worktree:**

1. **Canario.** `batuta canary --provider dsh --model deepseek-v4-flash` ejecuta de verdad
   y deja recibo completo. Es de sólo lectura: no hay worktree, no hay diff, no hay
   allowlist que verificar. Es la ruta crítica y todo lo demás se apoya en ella.
2. **Delegación.** Worktree, diff y verificación de alcance.

Partirlo así no es comodidad: el canario ejercita el proceso, el entorno, los ficheros de
corrida, el recibo y la procedencia **sin** la mitad más peligrosa. Si algo falla, falla
donde se puede mirar.

---

## §2 El recibo

Es el artefacto de la fase. Responde a `harness.py:454` del orquestador viejo, que
reportaba `"Harness worker failed with exit 1"` y tiraba stdout y stderr del hijo; esa
ceguera costó días de diagnóstico.

```
Recibo
├── encargo         id, manifiesto y su sha256, modelo pedido
├── invocación      argv REAL, cwd, entorno efectivo (nombres, nunca valores),
│                   ficheros de corrida materializados y su contenido
├── resultado       código de salida, stdout, stderr ÍNTEGRO, duración
├── procedencia     provider y model OBSERVADOS, ids de sesión, herramientas usadas
└── veredicto       verde | rojo, con el motivo nombrado
```

### Lo que la medición obliga a llevar, y no estaba en el plan

- **Los ficheros de corrida, con su contenido.** El modelo que corre no viaja en `argv`:
  viaja en un documento que batuta escribe. Un recibo que no lo incluya no permite
  reproducir la corrida ni explicar por qué corrió lo que corrió.
- **Los ids de sesión, en orden.** `SessionHeader` es inmutable y `parentSession` sólo lo
  escribe `fork()`, que headless no expone. **El índice de los intentos es el recibo, no
  dsh** (esquema §7).
- **Las herramientas usadas.** El registro anota cada `tool/call`. Un encargo que no declara
  `web_research` y cuyo registro muestra llamadas web es **recibo en rojo** (esquema §4 bis).
- **`provider` y `model` observados, no pedidos.** Se pidió `deepseek-v4-flash` tres veces y
  corrió otro modelo las tres. Un recibo que anota lo pedido miente sobre lo único que le da
  valor.

### Tres reglas duras

1. **stderr íntegro, siempre**, aunque el proceso saliera con 0. Y con `program` apuntando
   al enlace directo, no a `npx`: npx vuelca el argv completo —prompt incluido— en stderr.
2. **Procedencia ilegible = recibo en rojo.** El lector tolera una cola partida y descarta
   el último registro si está a medias; lo que **no** hace es rellenar el hueco con lo que se
   pidió. «No pude leerlo» y «no pasó nada» son cosas distintas.
3. **El veredicto nombra su motivo.** Nunca por subcadena sobre nuestra propia política: es
   R3, y ya se pagó una vez con un canario que devolvía `QUOTA_UNAVAILABLE` en 126 ms sin
   tocar la red porque leía su propio reflejo.

---

## §3 La ejecución

### El worktree es la identidad del encargo

`projectKey(cwd)` es determinista, así que **el worktree de un encargo es estable durante
todo el encargo, reintentos incluidos**. Un encargo = un worktree = un grupo navegable en
`dsh web`. La ruta debe ser corta y distinguible cerca del final: la normalización del `cwd`
es *«intentionally lossy»* y dos rutas largas pueden colisionar.

### Los ficheros de corrida van FUERA del worktree

En un directorio de corrida hermano. Son configuración de batuta, no material del encargo:
dentro del worktree aparecerían en el diff. La carga ya rechaza rutas absolutas o con `..`;
**la comprobación de que no caen dentro del worktree vive aquí**, porque es aquí donde se
conocen las dos rutas a la vez. Ése fue un error del esquema que destapó una delegación.

### El diff incluye lo no rastreado

Medido: un modelo aplicado, obedeciendo el sandbox, creó un proyecto de 65 MB fuera de las
rutas autorizadas. `git diff` a secas no lo habría visto.

> **Contención y alcance son cosas distintas.** El sandbox del proveedor confina al worktree
> entero y funciona. La allowlist es más fina y el proveedor no la conoce: sólo se puede
> verificar **sobre el resultado**.

### El proceso es el límite (R6)

Matar la tarea mata el árbol. `TaskStop` del sistema viejo dejaba al hijo vivo gastando
cuota. Se ejecuta en su propio grupo de procesos y se mata el grupo.

**Comprobado antes de escribir una línea de C, y con esto la pregunta queda cerrada.** Sonda
sobre rustc 1.98, edición 2024:

```
hijo lanzado: pid 1647897, líder de su propio grupo
nietos antes de matar: 2
kill al grupo: exit 0
nietos después: 0
```

`std::os::unix::process::CommandExt::process_group(0)` es **biblioteca estándar**: lanzar al
hijo como líder de su propio grupo no cuesta ninguna dependencia. Lo único que falta es
`killpg`, que `nix` envuelve sin `unsafe`.

Y el aislamiento fino —seccomp, capabilities— **no hace falta**, con un argumento que sólo
existe desde que medimos: el confinamiento del sistema de ficheros lo pone el propio
proveedor, y **consta en el registro de sesión** (`sandbox/mode: workspace-write`,
`permission/preset`). batuta no tiene que construir una jaula: tiene que declararla,
comprobar que se aplicó, y poseer el límite del proceso. Duplicarla desde fuera sería una
segunda jaula que nadie verifica.

**Nada baja a C en esta fase**, y no por preferencia: porque la medida dice que no hace
falta. Si algún día hiciera falta, el benchmark de la Fase 4 lo dirá antes.

---

## §3 bis Los leases: admisión por evidencia

Dos espacios de nombres, como pide la arquitectura: **por modelo** y **por repositorio**. Un
encargo toma los dos. **La inspección no toma ninguno**, y de ahí sale R9 gratis: leer el
directorio de leases es una lectura normal, así que `inventory` no puede hacer cola detrás
de una delegación. Dos `orchestrator_inventory` se fueron a segundo plano tras 120 s por
eso exactamente.

**Adquisición:** creación exclusiva (`O_EXCL`) de `<state>/leases/<espacio>/<clave>.lease`.
Quien pierde la carrera **no espera**: recibe `AdmissionUnavailable` nombrando al dueño
actual. El sistema viejo daba el error sin decir quién lo tenía, que es la mitad inútil del
mensaje.

**El contenido del lease es la prueba de vida de su dueño:**

```
task_id · pid · pgid · process_start_time · adquirido_en
```

### Caducidad por evidencia, nunca por antigüedad

Un lease se reclama **sólo si se puede demostrar que su dueño ya no existe**: el par
`(pid, process_start_time)` leído de `/proc/<pid>/stat` no coincide con el anotado.

Merece subrayarse porque dsh, ante el mismo problema, decidió lo contrario: *«a contender
times out without removing the existing lock because age cannot distinguish a crashed owner
from a paused live writer; orphan recovery is an operator action»*
(`dsh-settings-file/README.md`). **Tienen razón sobre la antigüedad**, y por eso batuta no
la usa. Usa una comprobación que sí distingue las dos cosas, y `process_start_time` cierra
el hueco de la reutilización de PID.

Es la doctrina de R3 aplicada a la admisión: no se decide por heurística, se decide mirando
el hecho.

**Liberación:** al soltar el guardián, y por muerte del dueño. Como el hijo corre en su
propio grupo de procesos y `killpg` lo mata entero, matar la tarea deja el lease
demostrablemente huérfano y el siguiente lo reclama sin intervención de nadie.

**Aviso de portabilidad:** `/proc` es de Linux. La prueba de vida vive en un módulo propio
para que el día que importe se sustituya en un solo sitio.

---

## §4 El canario, observacional

`expect = "token_echo"`: se genera un token irrepetible, se pide que lo devuelva exacto, y
se comprueba que **volvió ese token**. No se busca una subcadena en un veredicto propio, y
no se consulta la política que el canario debe informar (R3).

Un canario que falla dice **cuál** de las cinco cosas falló: el ejecutable no se resolvió, el
hash no cuadró, el proceso salió con error, la salida no traía el token, o la procedencia no
se pudo leer. Cinco motivos, cinco mensajes.

---

## §5 Criterio de aceptación

Binario:

1. `batuta canary --provider dsh --model deepseek-v4-flash` ejecuta y deja recibo con argv,
   código de salida y stderr íntegro.
2. El recibo anota `provider` y `model` **observados**, y no coinciden por casualidad: un
   manifiesto que pida un modelo distinto del que el documento de settings fija produce
   **recibo en rojo**.
3. `batuta canary --provider abacus` ejecuta ZAI GLM 5.3 Flash. **El segundo manifiesto es
   donde se descubre si la abstracción aguanta**, y cierra el fallo que originó el proyecto
   sin haber parcheado el núcleo.
4. **R6 completo:** matar la tarea deja **cero** procesos hijos (`pgrep -P`) **y cero
   leases**. Las dos mitades, porque la regla tiene dos mitades.
5. **R9:** listar los leases con una corrida de 60 s en curso responde al instante.
6. Un fichero de corrida que cayera dentro del worktree se rechaza antes de ejecutar.
7. `bash scripts_ci/local_gates.sh` en código 0.

El punto 2 es el que más importa y el que no estaba en el brief: es la única forma de
demostrar que el recibo no miente.
