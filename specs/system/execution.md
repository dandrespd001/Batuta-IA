# Ejecución, relevo y recibos vigentes

## CAP-EXECUTION — Ejecución durable

**Estado**: `implemented`. No tiene evolución funcional abierta en el roadmap de este inventario.

### REQ-EXECUTION-001 — Estado durable y una sola ruta activa

Cada run persiste petición, generación seleccionada, intentos, reservas, transiciones y próxima
acción antes de efectuar la llamada correspondiente. Un lease impide rutas activas concurrentes;
`invocation_started` sin resultado pasa a `outcome_unknown` y nunca se reenvía automáticamente.

El perfil de ejecución y cada grant son JSON cerrados y sellados. El grant fija manifest, rutas,
acciones, operaciones, caducidad y límites positivos; se intersecta con la generación vigente antes
de reservar y nunca se amplía por una generación posterior. El ledger reserva máximos antes de la
invocación, confirma consumo conocido y conserva completa una reserva ambigua.

### REQ-EXECUTION-002 — Retry y relevo acotados

Sólo un rate limit observado con plazo admisible reintenta la misma ruta. Los demás fallos conocidos
eligen, si procede, una ruta no intentada y autorizada usando un checkpoint con objetivo, fallo,
hechos, siguiente paso y presupuesto; nunca reenvían el historial completo. Espera e intento se
reservan antes de dormir.

La salud conserva las veinte observaciones más recientes, cuenta ambiguos como no exitosos y calcula
p95 por rango próximo. Cada actualización reemplaza sólo ese componente mediante CAS. Autenticación
y saldo bloquean hasta intervención; un rate limit sin plazo sigue su calendario de sondas.

### REQ-EXECUTION-003 — Recibos sellados y recuperables

El recibo terminal cerrado conserva petición, grant, candidatos, descartes, decisiones, reservas,
consumos, transiciones, resultados y checkpoints en orden, queda sellado y append-only, y puede
recuperarse tras reinicio sin reescribir bytes ni sobrescribir un ID existente.

El adaptador ejecuta exactamente una invocación derivada del manifest, con entorno allowlisted y
salida acotada. Durante todos los intentos el worktree es estable, el encargo empieza con una línea
identificadora y el recibo ordena los IDs de sesión observados; el proveedor conserva los hilos y el
recibo conserva su relación, sin fingir continuidad de chat que el harness no expone.

Las verificaciones de coordinación, relevo y recibos se ejecutan offline y están ancladas en el
registro.
