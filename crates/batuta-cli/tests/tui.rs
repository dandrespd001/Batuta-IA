//! Armazón TUI: cuatro vistas y distinción explícita de calidad.

use batuta_cli::{Layout, TuiApp, TuiView};

#[test]
fn la_tui_ofrece_catalogo_perfiles_evidencia_y_salud() {
    let mut app = TuiApp::new();
    assert_eq!(app.view(), TuiView::Catalog);
    assert!(app.snapshot(100).contains("Catálogo"));

    app.next_view();
    assert_eq!(app.view(), TuiView::Profiles);
    app.next_view();
    assert_eq!(app.view(), TuiView::Evidence);
    let evidence = app.snapshot(100);
    assert!(evidence.contains("investigado"));
    assert!(evidence.contains("override"));
    assert!(evidence.contains("efectivo"));
    assert!(evidence.contains("cobertura"));

    app.next_view();
    assert_eq!(app.view(), TuiView::Health);
    app.next_view();
    assert_eq!(app.view(), TuiView::Execution);
    let execution = app.snapshot(100);
    assert!(execution.contains("Perfil"));
    assert!(execution.contains("Grants"));
    assert!(execution.contains("Runs"));
}

#[test]
fn actualizar_investigacion_desde_tui_solo_crea_staging() {
    let root = std::env::temp_dir().join(format!(
        "batuta-tui-research-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let layout = Layout::under(root);
    let mut app = TuiApp::new();

    let request = app.queue_research_update(&layout).unwrap();

    assert!(app.snapshot(100).contains(&request));
    assert!(!layout.research().join("active.json").exists());
}
