# Quickstart de validación: adopción spec-anchored

## Prerrequisitos

- Checkout del repositorio en la rama de adopción.
- Python 3.11+, Bash 4.4+, Git 2.39+, Node 20+, GNU `sha256sum`/`timeout` y Rust 1.98
  (edición 2024) declarado por el workspace.
- Ninguna credencial ni acceso de red para los gates.

Spec Kit v1.0.2 es sólo herramienta de autoría y no es dependencia del binario. No necesita estar
instalado globalmente para ejecutar los gates. La comprobación oficial opcional puede necesitar la
copia fijada ya cacheada y se ejecuta fuera del gate permanente:

```sh
uvx --from git+https://github.com/github/spec-kit.git@v1.0.2 \
  specify --version
uvx --from git+https://github.com/github/spec-kit.git@v1.0.2 \
  specify integration status --json
```

Se comprueban por separado dos fuentes: `specify --version` debe informar `1.0.2`; el JSON de estado
debe declarar `status: ok`, integración `codex`, dos manifests y cero rutas o ficheros administrados
ausentes, modificados, inválidos o sin comprobar. El JSON de integración es la fuente oficial del
estado de SC-008; no se infiere de él la versión de la CLI.

El equivalente permanente y offline no ejecuta Spec Kit:

```sh
python3 scripts_ci/check_speckit_integration.py
```

Resultado esperado: versión `1.0.2`, integración `codex`, manifests/rutas válidos y SHA-256 exactos.
La ausencia de instalación global no modifica este resultado ni permite omitirlo.

## Validar contratos por separado

```sh
python3 scripts_ci/validate_spec_anchors.py
python3 scripts_ci/validate_tdd_evidence.py
python3 scripts_ci/check_modularity.py
python3 scripts_ci/check_architecture.py
python3 scripts_ci/check_speckit_integration.py
```

Resultado esperado: cinco códigos `0`; modularidad puede imprimir advertencias para módulos entre 400
y 499 líneas o deuda registrada por encima del límite.

Para medir el requisito temporal en `ubuntu-latest`, cada proceso se lanza y registra por separado:

```sh
timeout 5s python3 scripts_ci/validate_spec_anchors.py
timeout 5s python3 scripts_ci/validate_tdd_evidence.py
timeout 5s python3 scripts_ci/check_modularity.py
timeout 5s python3 scripts_ci/check_architecture.py
timeout 5s python3 scripts_ci/check_speckit_integration.py
```

No se suman aquí pruebas Rust, Node, `unittest` ni `local_gates.sh`. Cada fixture determinista se
ejecuta dos veces y se comparan bytes de `stdout`, `stderr` y código.

## Ejecutar pruebas de mutación de los gates

```sh
python3 -m unittest discover -s scripts_ci/tests -v
```

Resultado esperado: todos los casos dirigidos pasan, incluidos duplicados, huérfanos, campos
desconocidos, clasificación de impacto, hashes V1/V2 alterados, límites/excepciones modulares,
dependencias prohibidas e integridad de Spec Kit. Los diagnósticos y códigos exactos están en la
[matriz mínima](research.md#matriz-mínima-de-mutaciones).

## Verificar preservación V1

```sh
sha256sum -c docs/evidence/v1.sha256
```

Resultado esperado: `OK` para `tdd.jsonl`, `tdd.schema.json` y los cuatro snapshots. El validador de
evidencia comprueba además exactamente 19 registros legados.

## Ejecutar la aceptación completa

```sh
./scripts_ci/local_gates.sh
```

Resultado esperado: formato, `no_std`, Spec Kit offline, specs/anchors, evidencia, modularidad,
arquitectura, pruebas de gates, sidecar, Clippy y workspace tests terminan en verde sin red ni
proveedores reales.

Para probar correlación de deriva sobre una comparación concreta:

```sh
BATUTA_SPEC_BASE=7de68af2c9a36ba3dcc65971e4bba83231fb3855 ./scripts_ci/local_gates.sh
```

Resultado esperado: toda ruta cambiada desde el baseline pertenece a una capacidad registrada y los
paquetes de impacto cubren los requisitos correspondientes.

La referencia canónica completa es `7de68af2c9a36ba3dcc65971e4bba83231fb3855`. Los modos de base son:

```sh
python3 scripts_ci/validate_spec_anchors.py
python3 scripts_ci/validate_spec_anchors.py --base 7de68af2c9a36ba3dcc65971e4bba83231fb3855
python3 scripts_ci/validate_spec_anchors.py --base ref-inexistente
```

El primer comando valida estructura, emite una sola advertencia `GIT_DIFF_OMITTED` y devuelve `0` si
no hay otro fallo. El segundo añade el diff y devuelve `0` si todo está cubierto. El tercero devuelve
`2` con `GIT_BASE_UNRESOLVABLE`; una base explícita ausente en un clon superficial nunca se ignora. CI
usa `fetch-depth: 0`, requiere `BATUTA_SPEC_BASE` y llama la misma entrada agregada.

## Demostrar navegación

Cronometrar de forma independiente `CAP-MANIFESTS`, `CAP-STATE` y `CAP-ROLLOUT`:

1. Abrir `specs/anchors.json` e iniciar el reloj.
2. Localizar la capacidad y mostrar `owner_spec`, `status` y un requisito.
3. Seguir su prueba/gate o evidencia; un protocolo manual solo no vale para `implemented`.
4. Seguir `roadmap_id` hasta `ROADMAP.md`.
5. Detener el reloj al comprobar que no hay una segunda autoridad normativa.

Cada recorrido debe durar ≤ 5 minutos. Los destinos de cada caso están cerrados en
[research.md](research.md#protocolo-mínimo-de-navegación).

## Comprobar recuperación

- Inyectar un fallo antes de publicar una migración y comprobar que el activo conserva sus bytes, el
  backup es restaurable y repetir la misma entrada no duplica cambios.
- Dejar una fila de paridad incompleta y comprobar que el documento anterior conserva contenido y
  autoridad; sólo tras paridad total puede convertirse en enlace.
- Fallar la publicación coordinada de spec/anchors/roadmap y comprobar que ningún fichero parcial se
  presenta como estado activo.

## Revisión previa a implementación

`checklists/acceptance-evidence.md` contiene una fila por CHK001–CHK036. Antes de implementar se
comprueba que hay 36 filas, `requirements.md` sigue 16/16 y `acceptance.md` sigue 0/36. Sólo el revisor
humano puede aprobar; después de esa aprobación se cambian exclusivamente las 36 marcas y se repite
`$speckit-analyze`. Hasta entonces `$speckit-implement` permanece bloqueado.
