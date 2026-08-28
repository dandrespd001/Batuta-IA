//! Lo que cada error tiene que **nombrar**.
//!
//! «Falló» no es un diagnóstico. `"Harness worker failed with exit 1"` es
//! exactamente el mensaje que costó días: decía que algo había ido mal y no
//! decía qué, así que hubo que reconstruirlo desde fuera.
//!
//! Aquí se fija lo que el mensaje tiene que decir, no cómo lo dice.

use std::path::PathBuf;

use batuta_exec::ExecError;

/// El token del canario sale de `/dev/urandom`, que **no se lanza**: se lee.
///
/// La variante existe porque no existía. El primer cuerpo del canario tuvo que
/// meter ese fallo en `Spawn` con `/dev/urandom` de «programa», y el mensaje
/// habría dicho «no se pudo lanzar el programa /dev/urandom», que es falso. El
/// implementador lo señaló en vez de tragárselo; el vocabulario estaba
/// incompleto y el hueco se tapa aquí, no en el sitio del uso.
#[test]
fn el_token_que_no_se_pudo_generar_no_dice_que_algo_no_se_pudo_lanzar() {
    let error = ExecError::TokenSource {
        source: std::io::Error::new(std::io::ErrorKind::NotFound, "no hay /dev/urandom"),
    };
    let mensaje = error.to_string();

    assert!(mensaje.contains("token"), "no nombra el token: {mensaje}");
    assert!(
        !mensaje.contains("lanzar"),
        "leer una fuente de azar no es lanzar un programa: {mensaje}"
    );
    assert!(
        mensaje.contains("no hay /dev/urandom"),
        "pierde la causa: {mensaje}"
    );
}

/// El fallo de lanzamiento sí nombra el programa: es lo único que permite
/// distinguir «no está» de «está y no arranca».
#[test]
fn el_fallo_de_lanzamiento_nombra_el_programa() {
    let error = ExecError::Spawn {
        program: PathBuf::from("/usr/bin/inexistente"),
        source: std::io::Error::new(std::io::ErrorKind::NotFound, "no such file"),
    };

    assert!(error.to_string().contains("/usr/bin/inexistente"));
}
