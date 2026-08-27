//! El `TaskSpec` y sus invariantes.
//!
//! Un `TaskSpec` construido es un `TaskSpec` coherente: no existe la variante
//! «válido pero sin validar». Lo que se rellena es un [`TaskSpecDraft`]; lo que
//! circula por batuta es un [`TaskSpec`].

use std::collections::BTreeSet;

use batuta_contract::{
    Capability, OutputContract, Role, Sensitivity, TaskSpec, TaskSpecDraft, WriteMode,
};

fn borrador() -> TaskSpecDraft {
    TaskSpecDraft {
        schema_version: batuta_contract::SchemaVersion::CURRENT,
        role: Role::BoundedCpp,
        sensitivity: Sensitivity::Internal,
        output_contract: OutputContract::UnifiedDiff,
        write_mode: WriteMode::ValidatedPatch,
        allowed_write_paths: vec![
            "addons/chunsa_sim/core".parse().unwrap(),
            "tests".parse().unwrap(),
        ],
        required_capabilities: BTreeSet::from([Capability::Read]),
        gate_profile: "chunsa-determinism".parse().unwrap(),
        timeout_seconds: 1800,
        max_repairs: 2,
        reasoning_effort: None,
    }
}

#[test]
fn un_borrador_coherente_se_construye() {
    let spec = TaskSpec::try_from(borrador()).expect("el borrador es coherente");
    assert_eq!(spec.role(), Role::BoundedCpp);
    assert_eq!(spec.timeout_seconds(), 1800);
    assert_eq!(spec.allowed_write_paths().len(), 2);
}

/// Un modo que escribe **siempre** exige la capacidad `write`, lo diga el
/// borrador o no. Repetirlo a mano sólo crea sitios donde puedan divergir.
#[test]
fn escribir_implica_exigir_la_capacidad_write() {
    let spec = TaskSpec::try_from(borrador()).unwrap();
    assert!(spec.required_capabilities().contains(&Capability::Write));
    assert!(spec.required_capabilities().contains(&Capability::Read));
}

/// R2 — el fallo que la paga: `web_research` declarada en un solo modelo, su
/// transporte sin navegar, y una delegación de investigación que hizo **cero
/// llamadas a herramientas** y devolvió 38 KB con veinte citas.
#[test]
fn investigar_implica_exigir_web_research() {
    let mut draft = borrador();
    draft.role = Role::Research;
    draft.write_mode = WriteMode::ReadOnly;
    draft.output_contract = OutputContract::Text;
    draft.allowed_write_paths.clear();

    let spec = TaskSpec::try_from(draft).unwrap();
    assert!(
        spec.required_capabilities()
            .contains(&Capability::WebResearch)
    );
    assert!(!spec.required_capabilities().contains(&Capability::Write));
}

#[test]
fn read_only_no_admite_rutas_de_escritura() {
    let mut draft = borrador();
    draft.write_mode = WriteMode::ReadOnly;
    draft.output_contract = OutputContract::Review;

    let error = TaskSpec::try_from(draft).expect_err("read_only con allowlist es incoherente");
    let mensaje = error.to_string();
    assert!(mensaje.contains("read_only"), "{mensaje}");
    assert!(mensaje.contains("allowed_write_paths"), "{mensaje}");
}

/// R5 — la contención es por nombre. Una allowlist vacía en un modo que escribe
/// no significa «nada»: significa que nadie ha dicho qué se puede tocar.
#[test]
fn escribir_sin_allowlist_no_se_admite() {
    let mut draft = borrador();
    draft.allowed_write_paths.clear();

    let error = TaskSpec::try_from(draft).expect_err("escribir sin allowlist es incoherente");
    assert!(error.to_string().contains("allowed_write_paths"), "{error}");
}

#[test]
fn un_diff_exige_un_modo_que_escriba() {
    let mut draft = borrador();
    draft.write_mode = WriteMode::ReadOnly;
    draft.allowed_write_paths.clear();

    let error = TaskSpec::try_from(draft).expect_err("unified_diff en read_only es incoherente");
    let mensaje = error.to_string();
    assert!(mensaje.contains("unified_diff"), "{mensaje}");
}

#[test]
fn read_only_no_puede_exigir_la_capacidad_write() {
    let mut draft = borrador();
    draft.write_mode = WriteMode::ReadOnly;
    draft.output_contract = OutputContract::Review;
    draft.allowed_write_paths.clear();
    draft.required_capabilities.insert(Capability::Write);

    let error = TaskSpec::try_from(draft).expect_err("read_only exigiendo write es incoherente");
    assert!(error.to_string().contains("write"), "{error}");
}

#[test]
fn una_allowlist_no_se_repite_ni_se_solapa() {
    let mut duplicada = borrador();
    duplicada.allowed_write_paths = vec!["tests".parse().unwrap(), "tests".parse().unwrap()];
    assert!(TaskSpec::try_from(duplicada).is_err());

    let mut anidada = borrador();
    anidada.allowed_write_paths = vec![
        "addons".parse().unwrap(),
        "addons/chunsa_sim/core".parse().unwrap(),
    ];
    let error = TaskSpec::try_from(anidada).expect_err("una ruta dentro de otra es redundante");
    assert!(error.to_string().contains("addons"), "{error}");

    let mut vecinas = borrador();
    vecinas.allowed_write_paths = vec!["addons".parse().unwrap(), "addons_extra".parse().unwrap()];
    assert!(
        TaskSpec::try_from(vecinas).is_ok(),
        "'addons_extra' no está dentro de 'addons'"
    );
}

#[test]
fn el_timeout_tiene_limites() {
    for malo in [0, 86_401, u32::MAX] {
        let mut draft = borrador();
        draft.timeout_seconds = malo;
        let error = TaskSpec::try_from(draft).expect_err("timeout fuera de rango");
        assert!(error.to_string().contains("timeout_seconds"), "{error}");
    }
    for bueno in [1, 1800, 86_400] {
        let mut draft = borrador();
        draft.timeout_seconds = bueno;
        assert!(TaskSpec::try_from(draft).is_ok(), "{bueno}");
    }
}

/// La regla de reencaminamiento del brief §5 —«dos fallos seguidos y el trabajo
/// pasa a Codex o a Claude»— sólo puede dispararse si nadie puede pedir más de
/// dos reparaciones.
#[test]
fn no_se_puede_reparar_mas_de_dos_veces() {
    assert_eq!(TaskSpec::MAX_REPAIRS, 2);
    let mut draft = borrador();
    draft.max_repairs = 3;
    let error = TaskSpec::try_from(draft).expect_err("tres reparaciones esquivan el reencaminado");
    assert!(error.to_string().contains("max_repairs"), "{error}");
}

/// La trampa del INSTRUCTIVO: `repo`, `profile` y `prompt` están **prohibidos
/// dentro de `task`**. Un campo de más no se ignora en silencio.
#[test]
fn un_campo_desconocido_hace_fallar_la_carga() {
    let error = serde_json::from_str::<TaskSpec>(
        r#"{
            "role": "bounded_cpp",
            "sensitivity": "internal",
            "output_contract": "unified_diff",
            "write_mode": "validated_patch",
            "allowed_write_paths": ["tests"],
            "gate_profile": "chunsa-standard",
            "timeout_seconds": 900,
            "repo": "/home/adquiod/Imágenes/Project/CHUNSA001"
        }"#,
    )
    .expect_err("`repo` no va dentro de task");
    assert!(error.to_string().contains("repo"), "{error}");
}

#[test]
fn un_task_spec_hace_ida_y_vuelta_por_json() {
    let spec = TaskSpec::try_from(borrador()).unwrap();
    let texto = serde_json::to_string(&spec).unwrap();
    assert_eq!(spec, serde_json::from_str::<TaskSpec>(&texto).unwrap());
}

#[test]
fn el_json_del_instructivo_se_carga() {
    let spec: TaskSpec = serde_json::from_str(
        r#"{
            "task_type": "bounded_cpp",
            "sensitivity": "internal",
            "output_contract": "unified_diff",
            "timeout_seconds": 1800,
            "write_mode": "validated_patch",
            "allowed_write_paths": ["addons/chunsa_sim/core", "tests"],
            "gate_profile": "chunsa-determinism",
            "max_repairs": 2
        }"#,
    )
    .expect("es el TaskSpec del INSTRUCTIVO §5");

    assert_eq!(spec.role(), Role::BoundedCpp);
    assert_eq!(spec.allowed_write_paths().len(), 2);
}

/// 0.3 — la simetría que faltaba.
///
/// `Write` estaba guardada: un encargo `read_only` que la exigiera fallaba.
/// `WebResearch` no lo estaba, así que un `role = implementation` podía exigir
/// navegación y pasar sin queja — y entonces la política tendría que buscarle un
/// modelo con recibo de `web_research` para un trabajo que no investiga. Es la
/// misma grieta de R2 por el otro lado.
#[test]
fn exigir_web_research_sin_rol_de_investigacion_no_se_admite() {
    let mut draft = borrador();
    draft.role = Role::Implementation;
    draft.required_capabilities.insert(Capability::WebResearch);

    let error = TaskSpec::try_from(draft)
        .expect_err("implementation exigiendo web_research es incoherente");
    let mensaje = error.to_string();
    assert!(mensaje.contains("web_research"), "{mensaje}");
    assert!(mensaje.contains("implementation"), "{mensaje}");
}

#[test]
fn el_rol_research_si_puede_exigir_web_research_explicitamente() {
    let mut draft = borrador();
    draft.role = Role::Research;
    draft.write_mode = WriteMode::ReadOnly;
    draft.output_contract = OutputContract::Text;
    draft.allowed_write_paths.clear();
    draft.required_capabilities.insert(Capability::WebResearch);

    let spec = TaskSpec::try_from(draft).expect("research puede pedir lo que ya implica");
    assert!(
        spec.required_capabilities()
            .contains(&Capability::WebResearch)
    );
}
