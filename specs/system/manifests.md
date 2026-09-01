# Manifests y procedencia vigentes

## CAP-MANIFESTS — Manifests de proveedor

**Estado**: `implemented`. **Roadmap**: `RM-002` divide el módulo heredado sin cambiar su contrato.

### REQ-MANIFESTS-001 — Forma cerrada y materialización declarativa

Cada manifest declara versión, proveedor, ejecutable, invocación, entorno, modelos, canarios,
procedencia, sustituciones y ficheros de corrida aplicables. Campos, versiones, vocabularios, rutas
absolutas, escapes con `..` y formas ambiguas se rechazan al cargar con un error accionable. La
materialización deriva sólo del manifest y ocurre fuera del worktree.

### REQ-MANIFESTS-002 — Pin, hash y procedencia observada

La ejecución resuelve el programa y revisión declarados, verifica el SHA-256 fijado y distingue el
modelo solicitado del proveedor/modelo realmente observados. Una discrepancia o formato de sesión
desconocido produce recibo rojo; una procedencia no observable nunca inventa el modelo ejecutado.
Las herramientas del proveedor se aceptan sólo por hechos de canario y sesión, no por promesas.

El catálogo DSH se consulta mediante un sidecar JSONL cerrado que sólo enumera identidad, revisión,
modalidades, contexto, esfuerzos y costes declarados; no transmite credenciales, saldo, cuota ni
suscripción. El cliente limita entorno, tiempo y stdout/stderr y no usa `stream` para descubrir el
catálogo.

Los dos requisitos tienen pruebas existentes enlazadas desde anchors. La deuda de tamaño de
`manifest.rs` pertenece exclusivamente a `RM-002`.
