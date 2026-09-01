# Estado, política y routing vigentes

## CAP-POLICY — Política operativa

**Estado**: `partial`. **Roadmap**: `RM-004`.

### REQ-POLICY-001 — Política cerrada y explícita

La política activa es versionada, cerrada y sellada; aliases resuelven una ruta exacta y los límites,
perfiles, fallbacks y autorizaciones no nacen de defaults ocultos ni de datos aportados como
candidatos por el cliente.

El perfil operativo declara únicamente workdir canónico y límites de ejecución; importarlo sólo crea
staging y aplicarlo exige ID, hash esperado, base activa y sello coincidentes. La política mínima fija
intentos, espera máxima y handoffs sin valores implícitos.

### REQ-POLICY-002 — Propuesta y aplicación transaccional

**Estado parcial**: existen validación estricta y migración revisable, pero falta una frontera común
de `import/status/apply` para toda la política con staging, confirmación, CAS, backup y reintento
idempotente. Su aceptación pertenece a `RM-004`.

## CAP-ROUTING — Selección sellada

**Estado**: `implemented`. **Roadmap**: `RM-003` sólo divide módulos sin cambio funcional.

### REQ-ROUTING-001 — Elegibilidad y selección deterministas

El selector descarta por capacidad, sensibilidad, evidencia, contexto, esfuerzo, cooldown y
autorización; aplica calidad mínima y margen, minimiza coste esperado y desempata por latencia y
`RouteRef`. Todas las razones de descarte quedan estructuradas.

### REQ-ROUTING-002 — Decisión sellada contra una generación

Cada decisión sella la generación y los hashes ordenados de catálogo, política, evidencia, salud,
capacidades y recibos usados. El cliente no aporta candidatos, hashes, reloj ni clase interna.

## CAP-STATE — Estado generacional

**Estado**: `partial`. **Roadmap**: `RM-004`.

### REQ-STATE-001 — Publicación generacional atómica

El manifest de estado es la única raíz activa; referencia objetos canónicos inmutables por hash. Un
commit sincroniza objetos antes del manifest, publica por rename atómico y rechaza una base obsoleta,
de modo que un fallo conserva íntegra la generación anterior.

### REQ-STATE-002 — Migración completa y CAS común

**Estado parcial**: el estado y varios almacenes ya prueban CAS y recuperación, pero falta aplicar la
misma frontera a todas las migraciones V1 y escritores de política/estado, con backup restaurable y
reintento idempotente. Su aceptación pertenece a `RM-004`.

## Protocolos manuales de aceptación parcial

### Protocolo para REQ-POLICY-002

Precondiciones: paquete de `RM-004`, base activa conocida y fixture V1 recuperable. Preparar una
propuesta sin tocar el activo; comprobar diff y sello; intentar aplicar con ID incorrecto y base
obsoleta; aplicar con confirmación válida; inyectar fallo antes del publish. Se acepta sólo si los
rechazos no escriben, el éxito cambia una vez la generación y el fallo conserva activo y backup.

### Protocolo para REQ-STATE-002

Precondiciones: todos los escritores enumerados y una base común. Ejecutar dos escritores contra la
misma base, migración fallida y el mismo reintento dos veces. Se acepta sólo si como máximo un CAS
publica, el activo previo conserva bytes, el backup restaura y el reintento no duplica objetos ni
registros.
