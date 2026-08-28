# Fase 4 — Política, benchmark y retirada

**Estado: propuesta. No se construye nada de aquí sin aprobación del Arquitecto**, salvo
lo marcado como *medición*, que no decide nada y sirve para decidir.

La Fase 3 dejó recibos de verdad. Eso cambia lo que esta fase puede hacer: `batuta-policy`
decide sobre hechos, y hasta ahora no había ninguno.

---

## §1 Lo que la Fase 3 midió y esta fase hereda

Cuatro cosas medidas que aquí dejan de ser suposiciones:

1. **El tiempo está en el proveedor, no en batuta.** dsh tardó 2581 ms y abacus 16 405 ms
   en devolver una sola línea. Todo lo que batuta hace alrededor —cargar manifiestos,
   sustituir, materializar, lanzar, leer el registro— vive dentro de esos números.
2. **Un recibo caduca cuando cambia el manifiesto que lo produjo.** Por eso el recibo lleva
   `manifest_sha256` desde la Fase 3. No hizo falta inventarlo aquí.
3. **Hay proveedores cuyo modelo no es comprobable**, y el recibo lo dice
   (`model_confirmed`). La política tiene que poder enrutar a uno de ésos **sin fingir** que
   está confirmado.
4. **El stderr de abacus lleva `model: ZAI_GLM_5_3_FLASH`.** Su procedencia sí es legible,
   en otro sitio y otro formato. Es la primera prueba de que `provenance.source` quiere un
   tercer valor.

---

## §2 El benchmark va primero, y es medición

> «Nada baja a C "por eficiencia" sin un benchmark previo que enseñe el coste en el perfil»
> — regla del brief de arranque.

El benchmark **no decide**: mide, y luego se decide. Va primero porque su resultado cambia
lo que tiene sentido construir después.

Qué se mide, todo sobre trabajo real y no sintético:

| Tramo | Por qué importa |
|---|---|
| `ProviderManifest::load_dir` de `providers/` | R7 lo repite en **cada** invocación. Si costara, R7 sería caro |
| `resolve_argv` + `materialize` de dsh | dos documentos JSON por corrida |
| `parse_log` sobre un registro real | el de la delegación de la Fase 3 pesa **3,7 MB comprimidos, 4526 líneas**. Es el único tramo con orden de magnitud sospechoso |
| `LeaseStore::list` con N leases | R9 es una promesa de **latencia** |
| El total, contra los 2581 ms del canario de dsh | la fracción que batuta controla |

**Criterio de aceptación:** `docs/medidas/COSTE.md` con los números y una frase que diga si
algo justifica bajar a C. La hipótesis a refutar es que **nada lo justifica**: si el
proveedor tarda 2,5 s y batuta tarda microsegundos, el ejercicio termina ahí y se dice.

- [x] Banco con `std::time::Instant`, sin dependencia nueva (no hace falta `criterion` para
      distinguir microsegundos de segundos)
- [x] Cada tramo con su número y su tamaño de entrada
- [x] `docs/medidas/COSTE.md` con la conclusión escrita: nada justifica bajar a C
      (`7f22ed2`)

---

## §3 `batuta-store` — los recibos, consultables

Hoy los recibos se escriben y nadie los lee. La política los necesita para R2.

**La pregunta que tiene que contestar**, y es una sola:

> ¿Tiene el par (proveedor, modelo) un recibo **verde** producido por **este** manifiesto y
> **no caducado**?

Tres condiciones, y las tres importan:

- **verde** — un recibo rojo es evidencia de lo contrario;
- **producido por este manifiesto** — se compara `manifest_sha256` contra el del manifiesto
  cargado ahora. Un manifiesto editado invalida sus recibos **sin que nadie tenga que
  acordarse**, que es la única forma en que eso ocurre de verdad;
- **no caducado** — y aquí sí por edad, al revés que un lease.

**Por qué el lease caduca por evidencia y el recibo por edad, y no es una incoherencia.**
Un lease describe *algo que está pasando ahora* —un proceso vivo— y de eso hay evidencia
directa en `/proc`. Un recibo describe *algo que pasó* y cuya vigencia depende del mundo
exterior: la caché de `npx` se reescribe sola, una cuota se agota, un proveedor retira un
modelo. No hay `/proc` que consultar para eso. La edad es un sustituto pobre y es el único
honesto, así que el TTL va **declarado y visible**, no escondido en una constante.

- [x] `ReceiptStore::{open, latest_green}` sobre el directorio de la Fase 3. `save` no se
      duplicó: los recibos sólo los escribe el camino que los sella en `batuta-cli`
- [x] Invalidación por `manifest_sha256`, con test: editar el manifiesto invalida el recibo
- [x] TTL declarado, no constante mágica; y el estado «caducado» dice **cuándo** caducó
- [x] R9: leer no toma ningún cerrojo. El aserto es de tiempo
- [x] Un recibo ilegible **no** es un recibo ausente: `Lookup::unreadable` lo conserva

Implementado en `batuta-store` durante T3 de la Fase 5 (`dbf6ab3`), que adelantó esta
pieza porque el panel también necesitaba consultar la evidencia.

---

## §4 `batuta-policy` — el enrutador

Un `TaskSpec` entra, un `(proveedor, modelo)` sale, **o un error que dice por qué no**.

Las reglas que lo gobiernan, y cada una con su forma concreta aquí:

**R2 — nada se declara, se demuestra.** Una capacidad sin recibo de canario verde y vigente
**no es enrutable**. El fallo que lo paga: `web_research` figuraba en un solo modelo y su
transporte no navega; la delegación hizo cero llamadas a herramientas y produjo 38 KB con
veinte citas inventadas.

**R3 — la medición nunca consulta la política que informa.** `batuta-policy` **no puede
depender de `batuta-exec`**. Se comprueba en el `Cargo.toml`, y hay un test que lo lee: la
puerta circular devolvió su veredicto en 126 ms sin tocar la red porque el canario leía el
mismo fichero que él debía informar.

**R12 — un solo planificador para todas las entradas.** El CLI y el MCP llaman a la misma
función. No dos caminos que «hacen lo mismo»: la misma. Se comprueba con un test que enruta
la misma tarea por las dos superficies y exige el mismo resultado.

**R13 — una perilla que nadie fija es un error de compilación.** No un `Default`, no un
`Option` con `unwrap_or`. Si la política tiene un campo, alguien lo escribe o no compila.
`allow_experimental` se validaba y nadie la pasaba nunca, y era la única puerta de GLM 5.3.

**La decisión que hay que tomar aquí, y es del Arquitecto:** ¿un proveedor con
`model_confirmed: false` es enrutable? Las dos respuestas son defendibles y son distintas:

- **Sí, y el recibo lo dice.** Es lo que abacus permite hoy; negarlo lo dejaría fuera y
  abacus es el proveedor que originó el proyecto.
- **Sólo para sensibilidades bajas.** Un encargo `internal` puede correr sin confirmar el
  modelo; uno de más arriba, no.

*Recomendación:* la segunda, con el umbral **declarado en la política y no en el código**.
Es la que conserva la información en vez de gastarla, y `Sensitivity` ya es un vocabulario
cerrado con orden.

### Bloqueo descubierto al reanudar después de la Fase 5

La persistencia de la elección ya existe (`batuta-policy`, `2e4d60b`), pero el enrutador
todavía no puede cumplir R2 sin inventar información:

- `TaskSpec` exige un conjunto de `Capability` (`read`, `write`, `tools`, `web_research`);
- `ModelEntry` sólo declara `roles` y `max_sensitivity`;
- el recibo verde demuestra transporte, token, procedencia, herramientas **no declaradas**
  y alcance, pero no conserva qué capacidades positivas ejercitó y demostró;
- `ReceiptStore::latest_green` indexa por modelo/manifiesto, no por capacidad.

Por tanto, un canario verde de eco no prueba `write`, y uno verde de dsh no prueba
`web_research`. Tratarlo como si lo hiciera repetiría exactamente el fallo que paga R2.
Antes de implementar `TaskSpec → Route` hay que cerrar este contrato con tareas acotadas:

- [ ] **P4.1 (20–30 min):** añadir al recibo un conjunto explícito de capacidades
      demostradas, vacío para el canario básico; ida y vuelta JSON y mensaje legible
- [ ] **P4.2 (20–30 min):** definir canarios de capacidad que ejerciten una capacidad real;
      ningún manifiesto puede declararla demostrada sin ese escenario
- [ ] **P4.3 (20–30 min):** hacer que `ReceiptStore` consulte evidencia vigente por
      `(modelo, capacidad)` sin tomar cerrojo
- [ ] **P4.4 (20–30 min):** añadir a `Politica` el umbral explícito de sensibilidad para
      `model_confirmed: false`, con migración de esquema decidida antes de escribir código
- [ ] **P4.5 (20–30 min):** implementar el selector puro y sus razones de descarte; sólo
      después conectarlo a CLI y MCP por la misma función (R12)

- [ ] `TaskSpec` → `Route { provider, model, receipt }`, o error que enumera lo descartado
      **y por qué cada uno** (R8: un «no hay ruta» sin motivos obliga a adivinar)
- [ ] Test: capacidad sin recibo vigente ⇒ no enrutable, nombrando la capacidad
- [ ] Test: `batuta-policy` no depende de `batuta-exec` (se lee el `Cargo.toml`)
- [ ] Test: la misma tarea por CLI y por MCP da la misma ruta
- [ ] Cero `Default` en la estructura de política
- [ ] La regla sobre `model_confirmed`, **una vez decidida**, con su test

---

## §5 La suite de las catorce reglas

El brief la pide «sin red ni disco». Eso acota lo que puede ser, y conviene decirlo antes
de escribirla: **no todas las reglas se pueden probar sin tocar la máquina**. R6 es sobre
procesos, R11 sobre ficheros en disco, R4 sobre el stderr de algo que corrió.

Así que la suite es dos cosas, y las dos se dicen:

1. **`crates/batuta-policy/tests/reglas.rs`** — las reglas cuya sustancia es pura: R2, R3,
   R8, R10, R12, R13. Sin red y sin disco de verdad.
2. **`docs/REGLAS.md`** — la tabla completa de las catorce, cada una con **el test concreto
   que la cierra** y el fallo medido que la paga. Las ocho restantes ya tienen su prueba en
   el crate donde ocurren; lo que falta no es la prueba, es poder verlas juntas.

Una suite que fingiera probar R6 sin lanzar un proceso estaría probando otra cosa y
diciendo que prueba R6. Eso es exactamente lo que este proyecto persigue.

- [ ] `reglas.rs` con las seis puras
- [ ] `docs/REGLAS.md` con las catorce, cada una con su test por nombre
- [ ] Ninguna fila sin test: una regla sin prueba se marca **pendiente**, no se omite

---

## §6 `batuta-mcp` — la superficie

- **R7:** manifiestos y política se releen **por invocación**. Hoy el MCP del orquestador
  viejo carga el Python al arrancar y un cambio no aplica hasta reconectar.
- **R9:** la inspección no hace cola. Dos `orchestrator_inventory` se fueron a segundo plano
  tras 120 s por una delegación en curso.
- **La separación de poderes se conserva:** `accept` y `reject` **no** se exponen por MCP.
  Aplicar un parche es acto del Arquitecto, no del modelo que lo escribió; separar las
  superficies impide que un mismo agente escriba y se apruebe a sí mismo.

- [ ] Herramientas de inspección: proveedores, modelos, recibos, leases, simulación de ruta
- [ ] Simular una ruta **no ejecuta y no gasta**
- [ ] Test: con una corrida viva, la inspección responde en menos de un segundo
- [ ] Test: no existe ninguna herramienta MCP que aplique un parche

---

## §7 El ADR de retirada de `ai-orchestrator`

**Se redacta aquí y se aprueba fuera.** Un ADR es una decisión, y una decisión delegada deja
de tener dueño.

`AGENTS.md` y `CLAUDE.md` de CHUNSA mandan hoy `tools/chunsa_ai.sh` y el perfil `chunsa`.
El ADR debe cubrir, y sin las tres no está completo:

1. Qué reemplaza al ciclo `submit` → `artifact` → `accept`/`reject`.
2. Qué pasa con los vocabularios cerrados de `task_type` (18 roles) y `gate_profile`.
3. Qué se hace con `docs/INSTRUCTIVO_ORQUESTADOR.md`.

**Ninguno de esos ficheros se toca antes de que el ADR esté aprobado.**

- [ ] `docs/adr/ADR-001_RETIRADA_ORQUESTADOR.md` redactado, con las tres respuestas
- [ ] Presentado al Arquitecto. **Nada más.**

---

## §8 Orden, y por qué éste

```
benchmark  →  store  →  policy  →  suite de reglas  →  mcp  →  ADR
(medición)                                                     (decisión)
```

El benchmark va primero porque es medición y su resultado cambia lo que tiene sentido
construir. El store va antes que la política porque la política **pregunta** y hoy no hay a
quién. La suite va después de la política porque seis de sus reglas se prueban ahí. El MCP
va al final de lo construible porque es una superficie sobre algo que tiene que existir. Y
el ADR va el último porque es lo único que no se puede empezar sin haber terminado el resto:
propone retirar una herramienta, y hasta que la que la reemplaza funciona, la propuesta es
una opinión.

## §9 Riesgos

- **El TTL de los recibos es una perilla que se hereda.** Puesto corto, cada tarea dispara
  un canario y se paga cuota constantemente; puesto largo, se enruta a un proveedor que dejó
  de funcionar hace una semana. Va declarado y visible precisamente porque no hay valor
  obviamente correcto.
- **`batuta-mcp` reabre la puerta circular si se descuida.** El MCP es una superficie de
  *inspección* y de *lanzamiento*; en el momento en que informe de un estado que él mismo
  decide, R3 se rompe otra vez. La separación de crates es la defensa, y es estructural, no
  de disciplina.
- **El ADR toca ficheros de otro repositorio.** CHUNSA001 deniega `Edit` y `Write` en sus
  propios settings, así que el ADR se redacta desde fuera. No es una laguna: es la misma
  separación de poderes.
