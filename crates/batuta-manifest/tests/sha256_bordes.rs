//! R11 exige fijar el binario por versión **y hash**. El hash lo calcula una
//! implementación de SHA-256 escrita a mano dentro del crate, porque el árbol de
//! dependencias no traía ninguna.
//!
//! Que acierte con dos binarios reales de megabytes no basta: SHA-256 se rompe
//! en el **relleno**, y el relleno sólo se ejerce en los bordes de bloque. Con
//! 64 bytes de bloque y 8 de longitud, las longitudes 56..63 obligan a un bloque
//! extra; ahí es donde una implementación equivocada acierta en todo lo demás y
//! falla en silencio.
//!
//! Esta prueba también sobrevive a cambiar la implementación por un crate: sigue
//! comprobando lo mismo desde fuera.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use batuta_manifest::ProviderManifest;

/// SHA-256 de referencia, calculado por `sha256sum` del sistema.
fn sha256_de_referencia(ruta: &Path) -> String {
    let salida = std::process::Command::new("sha256sum")
        .arg(ruta)
        .output()
        .expect("sha256sum debe estar disponible");
    assert!(salida.status.success(), "sha256sum falló");
    String::from_utf8(salida.stdout)
        .expect("salida no UTF-8")
        .split_whitespace()
        .next()
        .expect("sha256sum no devolvió nada")
        .to_string()
}

fn manifiesto(programa: &Path, sha256: &str) -> String {
    format!(
        r#"schema_version = 1
id = "bordes"
kind = "cli"

[executable]
program       = "{p}"
version_pin   = "1.0"
version_probe = ["--version"]
sha256        = "{h}"
resolve       = ["{p}"]

[auth]
method = "oauth_cli"

[invoke]
argv    = ["{{prompt}}"]
workdir = "worktree"
prompt  = {{ via = "argv" }}

[env]
allow = ["HOME"]

[response]
parser = "plain_text"

[provenance]
source = "declared"

[[models]]
id              = "bordes-modelo"
route_model     = "remoto"
roles           = ["implementation"]
max_sensitivity = "internal"

[canary]
prompt = "Responde exactamente con: {{token}}"
expect = "token_echo"
"#,
        p = programa.display(),
        h = sha256
    )
}

#[test]
fn el_hash_acierta_en_todos_los_bordes_de_bloque() {
    let dir = std::env::temp_dir().join("batuta-sha-bordes");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("directorio temporal");

    // Vacío, un byte, y los seis bordes donde el relleno cambia de forma.
    for longitud in [0_usize, 1, 55, 56, 57, 63, 64, 65, 119, 120, 127, 128, 129] {
        let programa = dir.join(format!("prog{longitud}.bin"));
        let datos: Vec<u8> = (0..longitud)
            .map(|i| u8::try_from((i * 7 + 3) % 256).unwrap_or(0))
            .collect();
        fs::write(&programa, &datos).expect("escribir el programa de prueba");
        fs::set_permissions(&programa, fs::Permissions::from_mode(0o755))
            .expect("hacerlo ejecutable");

        let esperado = sha256_de_referencia(&programa);
        let ruta = dir.join(format!("borde{longitud}.toml"));
        fs::write(&ruta, manifiesto(&programa, &esperado)).expect("escribir el manifiesto");

        ProviderManifest::load(&ruta)
            .unwrap_or_else(|e| panic!("longitud {longitud}: el hash no cuadró: {e}"));
    }

    let _ = fs::remove_dir_all(&dir);
}

/// Y el otro lado: un hash equivocado tiene que rechazarse. Una comprobación que
/// nunca dice que no es una comprobación.
#[test]
fn un_hash_que_no_cuadra_se_rechaza() {
    let dir = std::env::temp_dir().join("batuta-sha-malo");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("directorio temporal");

    let programa = dir.join("prog.bin");
    fs::write(&programa, b"contenido cualquiera").expect("escribir");
    fs::set_permissions(&programa, fs::Permissions::from_mode(0o755)).expect("permisos");

    let ruta = dir.join("malo.toml");
    let mentira = "0".repeat(64);
    fs::write(&ruta, manifiesto(&programa, &mentira)).expect("escribir el manifiesto");

    let error = ProviderManifest::load(&ruta).expect_err("un hash falso no puede pasar");
    let mensaje = error.to_string();
    assert!(
        mensaje.contains(&mentira) || mensaje.contains("sha"),
        "{mensaje}"
    );

    let _ = fs::remove_dir_all(&dir);
}
