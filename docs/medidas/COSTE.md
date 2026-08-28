# Dónde va el tiempo

Medido el 28 de agosto de 2026 con `cargo run --release --example coste`, sobre trabajo
real: los manifiestos del repositorio, el registro de sesión más grande que este sistema ha
producido, y leases de verdad en disco.

Existe por una regla del brief de arranque:

> Nada baja a C «por eficiencia» sin un benchmark previo que enseñe el coste en el perfil.

**La vara es el canario real de dsh: 2581 ms de pared** (`CANARIOS.md`).

| Tramo | Entrada | Por vuelta | % del canario |
|---|---|---:|---:|
| `resolve_argv` de dsh | 5 argumentos | **596 ns** | 0,0000 % |
| `materialize` de dsh | 2 documentos JSON a disco | **19,10 µs** | 0,0007 % |
| `LeaseStore::list` | 50 leases vivos | **124,01 µs** | 0,0048 % |
| `load_dir` de `providers/` | 2 manifiestos, ~9 KB | **261,04 µs** | 0,0101 % |
| `zstd::decode_all` del registro | 1388 KB comprimidos | **9,95 ms** | 0,3857 % |
| `parse_log` del registro | 4526 líneas, 3652 KB crudos | **18,06 ms** | 0,6998 % |

---

## La conclusión, y es la que el ejercicio venía a refutar

**Nada justifica bajar a C.** Sumado, todo lo que batuta hace alrededor de una delegación
—cargar los manifiestos, sustituir, materializar, listar leases, descomprimir y recorrer el
registro entero— cuesta **menos del 1,2 % de una sola corrida**. El proveedor tarda dos
órdenes de magnitud más que el orquestador que lo llama.

Optimizar aquí no es prematuro: es **imperceptible**. Un tramo que se hiciera diez veces más
rápido devolvería 16 ms sobre 2581. Y el coste de escribir esos tramos en C —el `unsafe`, la
FFI, la superficie de aislamiento que habría que auditar— se paga entero.

Esto no cierra la pregunta para siempre. La cierra **con estos números y para estos
tamaños**, que es lo único que un banco puede hacer.

## Tres cosas que los números dicen y no se estaban buscando

**R7 es gratis.** Releer los manifiestos en *cada* invocación cuesta 261 µs. La
configuración en caliente no es un lujo que haya que racionar: es más barata que casi
cualquier forma de cachearla, y ahorra la clase entera de fallo en que un cambio no aplica
hasta reiniciar.

**R9 se cumple por cuatro órdenes de magnitud.** Listar 50 leases tarda 124 µs. La promesa
era «la inspección no hace cola», y el margen es tal que la inspección puede correr en medio
de cualquier cosa sin que se note. El fallo que la paga —dos `orchestrator_inventory` a
segundo plano tras 120 s— no era un problema de coste: era un cerrojo.

**El único tramo con orden de magnitud propio es leer la procedencia**, y aún así es 0,7 %.
De sus 18,06 ms, 9,95 son descompresión pura de zstd —es decir, más de la mitad del coste no
es de batuta, es del formato en que dsh guarda sus sesiones— y el resto es recorrer 4526
líneas de JSONL. Si algún día hiciera falta, el camino barato está a la vista: leer sólo la
cola del registro, que es donde están los eventos que interesan. **No hace falta hoy**, y
apuntarlo es distinto de hacerlo.

## Cómo reproducirlo

```sh
cargo run --release --example coste -p batuta-exec
```

El banco coge **el registro de sesión más grande** que haya bajo `$DSH_HOME/sessions`, no
uno cualquiera: lo que interesa medir es el peor caso que este sistema ha producido de
verdad. Si no hay ninguno, esa fila sale vacía y lo dice, en vez de inventar una entrada
sintética que mediría otra cosa.

Cada tramo se ejecuta una vez **antes** de arrancar el reloj. La primera vuelta paga el
disco frío y las páginas que aún no están, y eso no es el coste del tramo.

Sin `criterion`: para distinguir microsegundos de segundos no hace falta análisis
estadístico, y una dependencia de banco es una dependencia igual.
