# Reglas de ingeniería Rust

Estado: norma de proyecto.  
Fecha de adopción: 2026-08-31.

## Propósito

Estas reglas mantienen Batuta fácil de cambiar, revisar, auditar y depurar sin sacrificar
corrección. Se aplican a todo código Rust nuevo y a cada archivo existente que se modifique
de forma sustancial.

`DEBE` indica una obligación; `DEBERÍA`, la opción normal salvo motivo documentado; y
`PUEDE`, una elección contextual.

Las fuentes oficiales de Rust no fijan un máximo de líneas por archivo. Recomiendan agrupar
funcionalidad relacionada, separar responsabilidades distintas y limitar lo que una API
expone. Por eso, los números de este documento son **señales de diseño y revisión**, no una
meta que justifique fragmentar código cohesivo.

## Regla principal

Cada módulo DEBE tener una responsabilidad describible en una frase y una razón principal
para cambiar. Una operación debe poder leerse desde una API pequeña hacia detalles privados,
sin conocer simultáneamente persistencia, transporte, presentación y política.

La prioridad es:

1. corrección, seguridad e invariantes;
2. cohesión y fronteras claras;
3. legibilidad y facilidad de prueba;
4. tamaño.

Reducir líneas mediante macros opacas, expresiones densas o módulos diminutos que obligan a
saltar entre archivos incumple la regla aunque mejore una métrica.

## Tamaño y momento de dividir

| Unidad | Zona normal | Señal de revisión | Acción esperada |
|---|---:|---:|---|
| Función o método | hasta 40 líneas lógicas | más de 60 | extraer pasos con nombre; a partir de 100, dividir o justificar un `allow` local |
| Módulo de producción | hasta 300 líneas útiles | más de 400 | revisar responsabilidades; a partir de 500, dividir o registrar la excepción |
| Módulo de pruebas | hasta 500 líneas útiles | más de 700 | separar por comportamiento, no por cantidad arbitraria |
| Parámetros de función | hasta 5 | más de 7 | introducir un tipo de entrada validado o un constructor |
| Anidamiento | hasta 3 niveles | más de 3 | usar retornos tempranos o extraer una decisión |

No cuentan para decidir una división los comentarios útiles ni el formato automático. Sí
cuentan la cantidad de conceptos, efectos, estados y motivos de cambio.

Se DEBE dividir antes del umbral cuando aparezca cualquiera de estas señales:

- el módulo mezcla reglas de dominio con E/S, serialización o interfaz de usuario;
- una función cambia entre varios niveles de abstracción;
- las pruebas requieren preparar dependencias que el comportamiento no usa;
- una modificación habitual obliga a tocar regiones independientes del mismo archivo;
- los nombres genéricos (`data`, `manager`, `utils`, `process`) esconden varias funciones;
- comprender un error exige mantener demasiados estados simultáneos en la cabeza.

No se DEBE dividir cuando las partes comparten una sola invariante y separarlas la ocultaría,
o cuando el resultado sólo sería una cadena de archivos de una función sin frontera estable.

## Paquetes, crates y módulos

- La organización DEBE seguir capacidades del dominio, no tipos técnicos genéricos.
- `lib.rs`, `main.rs` y los archivos raíz de un módulo DEBERÍAN declarar, ensamblar y
  reexportar; no contener flujos operativos largos.
- Los elementos son privados por defecto. Se usa `pub(super)` o `pub(crate)` antes que `pub`
  cuando el consumidor no está fuera del crate.
- La API pública DEBE ser menor que la implementación y expresar sólo invariantes estables.
- Un nuevo crate requiere una frontera real de dependencia, compilación, portabilidad o
  propiedad. No se crea un crate solamente para reducir un archivo.
- Las dependencias entre crates DEBEN apuntar hacia contratos y dominio; una capa inferior no
  puede depender de CLI, TUI ni de un adaptador concreto.
- Las decisiones puras DEBERÍAN estar separadas de reloj, sistema de archivos, procesos, red y
  espera. Los efectos se inyectan en la frontera que los ejecuta.
- Un trait se introduce cuando existe más de una implementación real, una frontera externa o un
  doble de prueba necesario. No se crean traits preventivos para cada struct.
- Un módulo `utils` genérico está prohibido. El código compartido recibe el nombre del concepto
  que representa.

## Funciones, tipos y propiedad

- Una función DEBE operar en un solo nivel de abstracción y producir un resultado identificable.
- Se prefieren guard clauses y `?` a bloques profundamente anidados.
- Los nombres DEBEN explicar intención; los comentarios explican el porqué, una invariante o
  una decisión no evidente, nunca traducen la sintaxis.
- Los estados cerrados se representan con `enum`; identificadores, unidades y valores validados
  usan newtypes. No se intercambian conceptos mediante `String`, enteros o booleanos ambiguos.
- Una entrada compleja usa un tipo de petición cerrado y validado. Se evitan listas largas de
  parámetros y argumentos booleanos.
- Los valores inválidos DEBERÍAN ser imposibles de construir. Si no es posible, se validan una
  vez en el borde y el dominio conserva la garantía.
- Se toma prestado (`&T`, `&str`, slices) cuando basta; `clone()` requiere una necesidad de
  propiedad clara. No se sacrifica claridad por eliminar una copia irrelevante y medida.
- Las abstracciones cero-coste son un medio, no un objetivo. Un bucle explícito es preferible a
  una cadena de iteradores si comunica mejor el control o el manejo de errores.
- Los tipos públicos implementan los traits comunes que tengan semántica real, en especial
  `Debug`, igualdad y orden cuando correspondan.

## Errores, pánicos y `unsafe`

- Los fallos recuperables usan `Result<T, E>` con errores tipados. Las decisiones de dominio no
  se deducen analizando mensajes de texto.
- Los errores conservan su causa y añaden contexto en el borde donde ese contexto se conoce.
- `panic!`, `unwrap()` y `expect()` no representan entradas, red, disco ni fallos operativos.
  Sólo se admiten para una invariante demostrada o en pruebas; `expect()` explica la invariante.
- La documentación pública enumera `# Errors`, `# Panics` y `# Safety` cuando corresponda.
- El valor por defecto es `#![forbid(unsafe_code)]`. Una excepción requiere aislamiento mínimo,
  una prueba de por qué se cumplen las obligaciones y un comentario `SAFETY:` junto al bloque.

## Efectos, concurrencia y persistencia

- No se mantiene un lock durante E/S, espera o `.await` salvo que la exclusión sea precisamente
  la invariante documentada.
- El trabajo bloqueante no se ejecuta en un runtime asíncrono sin una frontera apropiada.
- Canales, colas, buffers, salidas y tareas concurrentes DEBEN tener límites y una política de
  cancelación o backpressure.
- Timeout, retry e idempotencia son decisiones explícitas; no se ocultan en helpers genéricos.
- Un estado durable se publica atómicamente y conserva suficiente journal para distinguir
  «no ocurrió» de «resultado desconocido» después de un crash.
- Reloj, sleeper, ejecutor y fuentes externas se inyectan donde haga falta probar recuperación,
  deadlines o concurrencia de manera determinista.

## Contratos y serialización

- Un contrato externo cerrado usa versión de esquema y rechaza campos desconocidos.
- Deserializar no evita la validación: el mismo constructor valida entradas de CLI, TUI, fichero
  y pruebas.
- Los hashes y sellos se calculan sobre una representación canónica definida, nunca sobre orden
  accidental de mapas o formato visual.
- Los campos que sostienen invariantes permanecen privados o sólo se construyen mediante una
  propuesta validada.
- Cambiar una API pública, un esquema, una feature o el MSRV requiere evaluar compatibilidad y
  documentar la migración.

## Pruebas y documentación

- Toda corrección o comportamiento nuevo empieza con una prueba roja reproducible.
- Las pruebas unitarias cubren decisiones pequeñas y detalles privados; las de integración
  ejercen la API pública y sus fronteras.
- Cada prueba verifica un comportamiento reconocible. Los helpers de pruebas se organizan por
  dominio y no esconden la acción que se está comprobando.
- Se prueban caminos de error, límites, recuperación, concurrencia e invariantes; no sólo el caso
  feliz ni porcentajes de cobertura.
- Las pruebas son deterministas y offline. CI usa ejecutables y relojes falsos, nunca cuota ni
  credenciales de proveedores.
- Cada crate explica en `//!` su responsabilidad, invariantes principales y dependencias
  permitidas. Todo elemento público tiene rustdoc proporcional a su riesgo.
- Los ejemplos públicos importantes son doctests y usan `?` para errores recuperables.
- Una decisión transversal o difícil de revertir se registra en un ADR breve; no se entierra en
  un comentario ni en el historial de chat.

## Formato, dependencias y gates

- Se usa `rustfmt` con la edición del workspace y ancho de 100 columnas. No se formatea a mano
  contra el resultado de la herramienta.
- Las dependencias y features nuevas requieren un consumidor actual, licencia compatible y una
  justificación de por qué la biblioteca estándar o el código existente no bastan.
- El workspace centraliza versión, MSRV, dependencias compartidas y lints.
- `clippy::pedantic` se aplica con criterio. No se habilita en bloque `clippy::restriction`; se
  seleccionan lints concretos cuando prueban una propiedad útil.
- Un `#[allow(...)]` nuevo DEBE ser local, nombrar el lint y explicar la excepción. Un allow a
  nivel de crate exige una decisión arquitectónica documentada.

Antes de integrar, deben pasar los gates del repositorio, como mínimo:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

También se mantienen los gates propios de Batuta: contrato `no_std`, evidencia TDD, sidecar
offline, concurrencia multiproceso y `scripts_ci/local_gates.sh`.

## Lista de comprobación de revisión

- [ ] La responsabilidad de cada módulo se puede decir en una frase.
- [ ] El cambio no mezcla dominio, E/S y presentación.
- [ ] La API nueva es la mínima necesaria y mantiene privados sus detalles.
- [ ] Las funciones largas, muchos argumentos y anidamiento fueron divididos o justificados.
- [ ] Los tipos hacen explícitos estados, unidades e invariantes.
- [ ] Los errores son tipados y no hay `unwrap()` operativo.
- [ ] Los efectos tienen límites, timeout y recuperación definidos.
- [ ] Las pruebas fallaban antes del cambio y cubren el camino adverso relevante.
- [ ] La documentación y los contratos cambiaron junto con el código.
- [ ] `fmt`, Clippy, tests y gates locales terminan en verde.

## Excepciones y mejora gradual

Estas reglas no obligan a refactorizar todo archivo heredado antes de corregirlo. Un cambio DEBE
dejar la zona tocada igual o más clara. Si la división segura excede el alcance, se registra una
deuda concreta con archivo, responsabilidad a extraer y prueba de caracterización necesaria en
[`DEUDA_MODULAR_RUST.md`](DEUDA_MODULAR_RUST.md).

Una excepción de tamaño es válida cuando mantener unido el código protege una invariante o mejora
la navegación. Debe constar junto al `allow` o en un ADR y revisarse cuando cambie la
responsabilidad. «Ya era largo» y «funciona» no son justificaciones.

## Fuentes primarias

- [The Rust Programming Language: paquetes, crates y módulos](https://doc.rust-lang.org/book/ch07-00-managing-growing-projects-with-packages-crates-and-modules.html)
- [The Rust Programming Language: separar módulos en archivos](https://doc.rust-lang.org/stable/book/ch07-05-separating-modules-into-different-files.html)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) y su
  [checklist](https://rust-lang.github.io/api-guidelines/checklist.html)
- [Rust Style Guide](https://doc.rust-lang.org/style-guide/)
- [Clippy: uso y configuración](https://doc.rust-lang.org/stable/clippy/usage.html) e
  [índice de lints](https://rust-lang.github.io/rust-clippy/stable/index.html)
- [The Rust Programming Language: manejo de errores](https://doc.rust-lang.org/stable/book/ch09-00-error-handling.html)
- [The Rust Programming Language: organización de pruebas](https://doc.rust-lang.org/book/ch11-03-test-organization.html)
- [Rustdoc: cómo escribir documentación](https://doc.rust-lang.org/rustdoc/how-to-write-documentation.html)
- [Cargo Workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html),
  [compatibilidad SemVer](https://doc.rust-lang.org/cargo/reference/semver.html) y
  [`rust-version`](https://doc.rust-lang.org/cargo/reference/rust-version.html)
- [Rust Reference: obligaciones de `unsafe`](https://doc.rust-lang.org/stable/reference/unsafe-keyword.html)
