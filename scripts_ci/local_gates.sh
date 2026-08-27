#!/usr/bin/env bash
# Gates permanentes de batuta (brief §7).
#
# Compilación local con prioridad baja y dos jobs, como manda AGENTS.md.
# `BATUTA_BUILD_JOBS` es un override consciente, no un atajo por defecto.
#
# No declara éxito sin registrar el comando y su resultado.

set -euo pipefail

cd "$(dirname "$0")/.."

JOBS="${BATUTA_BUILD_JOBS:-2}"
NICE=(nice -n 19)
FALLOS=0

ejecutar() {
    local titulo="$1"
    shift
    printf '\n== %s ==\n$ %s\n' "$titulo" "$*"
    if "$@"; then
        printf '   -> OK: %s\n' "$titulo"
    else
        local codigo=$?
        printf '   -> FALLO (exit %d): %s\n' "$codigo" "$titulo"
        FALLOS=$((FALLOS + 1))
    fi
}

# Cero E/S no es una promesa del brief, es un atributo del crate. Si alguien lo
# quita, `std::fs` vuelve a estar disponible y nadie se entera: por eso se
# comprueba aquí y no sólo en la revisión.
comprobar_no_std() {
    grep -qx '#!\[no_std\]' crates/batuta-contract/src/lib.rs
}

ejecutar "formato"            cargo fmt --all --check
ejecutar "cero E/S (no_std)"  comprobar_no_std
ejecutar "clippy"             "${NICE[@]}" cargo clippy --workspace --all-targets --jobs "$JOBS" -- -D warnings
ejecutar "tests"              "${NICE[@]}" cargo test --workspace --jobs "$JOBS"

printf '\n== resumen ==\n'
if [[ "$FALLOS" -eq 0 ]]; then
    printf 'todos los gates en verde\n'
else
    printf '%d gate(s) en rojo\n' "$FALLOS"
    exit 1
fi
