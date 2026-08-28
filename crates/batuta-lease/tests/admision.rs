//! Las reglas de admisión, una por prueba.
//!
//! La que da nombre al crate: **un lease caduca por evidencia, nunca por
//! antigüedad**. Hay dos pruebas que la fijan desde los dos lados —dueño muerto
//! que se reclama, dueño vivo y viejo que no—, porque una sola dejaría pasar la
//! implementación equivocada.

use std::fs;
use std::path::PathBuf;

use batuta_lease::{LeaseError, LeaseRecord, LeaseSpace, LeaseStore, Owner};

fn almacen(nombre: &str) -> (LeaseStore, PathBuf) {
    let raiz = std::env::temp_dir().join(format!("batuta-leases-{nombre}"));
    let _ = fs::remove_dir_all(&raiz);
    let store = LeaseStore::open(&raiz).expect("el almacén debe abrirse");
    (store, raiz)
}

/// Planta un lease a mano, con el dueño que se quiera. Usa `path_for`, que es
/// contrato público justamente para esto.
fn plantar(store: &LeaseStore, space: LeaseSpace, key: &str, owner: Owner, acquired_at: u64) {
    let registro = LeaseRecord {
        space,
        key: key.to_string(),
        task_id: "encargo-plantado".to_string(),
        owner,
        acquired_at,
    };
    let ruta = store.path_for(space, key);
    fs::create_dir_all(ruta.parent().expect("tiene padre")).expect("crear directorio");
    fs::write(&ruta, serde_json::to_vec(&registro).expect("serializa")).expect("escribir");
}

#[test]
fn un_lease_libre_se_toma() {
    let (store, _raiz) = almacen("libre");
    let guardian = store
        .acquire(LeaseSpace::Model, "dsh-deepseek-v4-flash", "encargo-1")
        .expect("nadie lo tenía");
    assert_eq!(guardian.record().task_id, "encargo-1");
}

/// El sistema viejo devolvía `AdmissionUnavailable` a secas. Saber que algo está
/// ocupado sin saber quién lo ocupa no permite hacer nada al respecto.
#[test]
fn el_segundo_encargo_es_rechazado_y_el_error_nombra_al_dueno() {
    let (store, _raiz) = almacen("ocupado");
    let _primero = store
        .acquire(LeaseSpace::Model, "dsh-deepseek-v4-flash", "encargo-1")
        .expect("el primero pasa");

    let error = store
        .acquire(LeaseSpace::Model, "dsh-deepseek-v4-flash", "encargo-2")
        .expect_err("el segundo no puede pasar");

    match &error {
        LeaseError::AdmissionUnavailable { key, held_by, .. } => {
            assert_eq!(key, "dsh-deepseek-v4-flash");
            assert_eq!(held_by.task_id, "encargo-1");
        }
        otro => panic!("se esperaba AdmissionUnavailable: {otro:?}"),
    }
    assert!(error.to_string().contains("encargo-1"), "{error}");
}

/// Dos modelos distintos en dos repositorios distintos deben poder trabajar a la
/// vez. Es el caso que el sistema viejo bloqueaba de más.
#[test]
fn dos_claves_distintas_no_se_estorban() {
    let (store, _raiz) = almacen("independientes");
    let _a = store
        .acquire(LeaseSpace::Model, "dsh-deepseek-v4-flash", "encargo-1")
        .expect("uno");
    let _b = store
        .acquire(LeaseSpace::Model, "abacus-glm-5.3-flash", "encargo-2")
        .expect("el otro modelo no está ocupado");
    let _c = store
        .acquire(LeaseSpace::Repository, "/tmp/otro-repo", "encargo-2")
        .expect("otro espacio, otra cosa");
}

/// La mitad de R6 que no es matar procesos: **matar la tarea libera el lease**.
#[test]
fn un_lease_cuyo_dueno_murio_se_reclama() {
    let (store, _raiz) = almacen("huerfano");

    // Un proceso que ya terminó: su pid ya no es él.
    let difunto = std::process::Command::new("/bin/true")
        .spawn()
        .expect("lanzar");
    let pid = difunto.id();
    let mut difunto = difunto;
    difunto.wait().expect("esperar");

    plantar(
        &store,
        LeaseSpace::Model,
        "modelo-x",
        Owner {
            pid,
            pgid: pid,
            start_time: 1,
        },
        0,
    );

    let guardian = store
        .acquire(LeaseSpace::Model, "modelo-x", "encargo-nuevo")
        .expect("un lease huérfano se reclama");
    assert_eq!(guardian.record().task_id, "encargo-nuevo");
}

/// El otro lado, y es el que impide la implementación fácil y equivocada: un
/// dueño **vivo** no pierde su lease por viejo que sea.
///
/// dsh rehúsa reclamar por antigüedad y tiene razón; batuta no reclama por
/// antigüedad tampoco. Reclama por evidencia, que es otra cosa.
#[test]
fn un_lease_viejo_con_dueno_vivo_no_se_reclama() {
    let (store, _raiz) = almacen("viejo-vivo");
    let vivo = Owner::current().expect("este proceso existe");

    plantar(&store, LeaseSpace::Model, "modelo-y", vivo, 0); // época: viejísimo

    let error = store
        .acquire(LeaseSpace::Model, "modelo-y", "encargo-oportunista")
        .expect_err("el dueño sigue vivo: la antigüedad no cuenta");
    assert!(
        matches!(error, LeaseError::AdmissionUnavailable { .. }),
        "{error:?}"
    );
}

/// El hueco que cierra `start_time`: un pid reutilizado no resucita un lease.
#[test]
fn el_mismo_pid_con_otro_arranque_es_un_huerfano() {
    let (store, _raiz) = almacen("pid-reutilizado");
    let mut yo = Owner::current().expect("este proceso existe");
    yo.start_time = yo.start_time.wrapping_add(9_999); // el pid vive, pero no es él

    plantar(&store, LeaseSpace::Model, "modelo-z", yo, 0);

    store
        .acquire(LeaseSpace::Model, "modelo-z", "encargo-nuevo")
        .expect("mismo pid, otro proceso: el lease es huérfano");
}

/// R9: la inspección nunca hace cola. Listar con un lease tomado tiene que
/// responder, no bloquearse.
#[test]
fn listar_no_toma_cerrojo_y_ve_lo_tomado() {
    let (store, _raiz) = almacen("inspeccion");
    let _guardian = store
        .acquire(LeaseSpace::Model, "dsh-deepseek-v4-flash", "encargo-1")
        .expect("tomado");

    let vistos = store
        .list(LeaseSpace::Model)
        .expect("listar no puede fallar");
    assert_eq!(vistos.len(), 1);
    assert_eq!(vistos[0].task_id, "encargo-1");
    assert!(vistos[0].is_held(), "su dueño somos nosotros, que vivimos");
}

/// Un lease que hay que acordarse de liberar es un lease que algún día no se
/// libera. Por eso es un guardián.
#[test]
fn soltar_el_guardian_libera_el_lease() {
    let (store, _raiz) = almacen("soltar");
    {
        let _guardian = store
            .acquire(LeaseSpace::Model, "modelo-w", "encargo-1")
            .expect("tomado");
        assert_eq!(store.list(LeaseSpace::Model).expect("listar").len(), 1);
    }
    assert!(
        store.list(LeaseSpace::Model).expect("listar").is_empty(),
        "al soltarlo, el lease desaparece"
    );

    store
        .acquire(LeaseSpace::Model, "modelo-w", "encargo-2")
        .expect("y otro puede tomarlo");
}
