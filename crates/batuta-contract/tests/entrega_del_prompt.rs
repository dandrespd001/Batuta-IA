//! Cómo llega el prompt al ejecutable, y cuánta sensibilidad admite cada vía.
//!
//! **Medido el 2026-08-27** contra dsh `0.1.1-rc.2`:
//!
//! ```text
//! $ echo "hola por stdin" | dsh --profile headless
//! error: a task is required, for example: dsh --profile headless "run the tests"
//! (exit 1, stdout 0 B)
//! ```
//!
//! dsh no lee el prompt por entrada estándar y no tiene bandera de fichero: la
//! tarea son los argumentos posicionales. Sin la variante `argv` no se puede
//! escribir `providers/dsh.toml`, así que el vocabulario se quedaba corto contra
//! el único transporte que hoy está demostrado.
//!
//! Y la variante no viene sola. `argv` es visible en `ps` para cualquier proceso
//! del mismo usuario, cosa que ni `stdin` ni un fichero en modo 0600 son. Esa
//! asimetría es una regla de política, y vive aquí para que ni el manifiesto ni
//! la política tengan que volver a deducirla —el mismo motivo por el que las
//! capacidades implícitas se derivan en un solo sitio.

use batuta_contract::{PromptDelivery, Sensitivity};

/// El hueco que destapó la medición.
#[test]
fn el_prompt_puede_viajar_por_argv() {
    let via: PromptDelivery = "argv".parse().expect("dsh sólo acepta la tarea por argv");
    assert_eq!(via, PromptDelivery::Argv);
    assert_eq!(via.as_str(), "argv");
}

/// `ps` es la razón, y el techo es `internal`.
#[test]
fn argv_no_admite_material_por_encima_de_internal() {
    assert_eq!(
        PromptDelivery::Argv.max_sensitivity(),
        Sensitivity::Internal
    );

    assert!(PromptDelivery::Argv.admits(Sensitivity::Public));
    assert!(PromptDelivery::Argv.admits(Sensitivity::Internal));

    for prohibida in [
        Sensitivity::Confidential,
        Sensitivity::Secrets,
        Sensitivity::FinancialControl,
        Sensitivity::Deployment,
    ] {
        assert!(
            !PromptDelivery::Argv.admits(prohibida),
            "argv es visible en ps: no puede llevar {prohibida}"
        );
    }
}

/// Las dos vías que no se ven desde fuera llegan hasta arriba de la escala.
#[test]
fn stdin_y_fichero_admiten_la_escala_entera() {
    for via in [PromptDelivery::Stdin, PromptDelivery::File] {
        assert_eq!(
            via.max_sensitivity(),
            Sensitivity::Deployment,
            "{via} debería admitir toda la escala"
        );
        for nivel in Sensitivity::ALL {
            assert!(via.admits(*nivel), "{via} debería admitir {nivel}");
        }
    }
}
