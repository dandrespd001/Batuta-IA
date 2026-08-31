//! TUI local con ratatui/crossterm, sin servidor.

use std::io;

use batuta_routing::ApplicationService;
use crossterm::event::{DisableBracketedPaste, EnableBracketedPaste};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::{ApiError, Layout, ResearchScope, queue_research_update, route_json};

mod execution;
mod interaction;
mod terminal;

use execution::ExecutionPanelState;
pub use execution::{RunPreviewV2, TuiExecutionJob, TuiExecutionSection, TuiExecutionWorker};
pub use interaction::TuiInputAction;
use interaction::TuiInteractionState;

/// Las vistas del contrato TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TuiView {
    /// Harnesses, rutas y alias.
    Catalog,
    /// Perfiles, pesos, costes y fallbacks.
    Profiles,
    /// Investigación, overrides y staging.
    Evidence,
    /// Cooldown, procedencia y recibos.
    Health,
    /// Perfil operativo, grants y corridas K4.
    Execution,
}

impl TuiView {
    const ALL: [Self; 5] = [
        Self::Catalog,
        Self::Profiles,
        Self::Evidence,
        Self::Health,
        Self::Execution,
    ];

    const fn title(self) -> &'static str {
        match self {
            Self::Catalog => "Catálogo",
            Self::Profiles => "Perfiles",
            Self::Evidence => "Evidencia",
            Self::Health => "Salud",
            Self::Execution => "Execution",
        }
    }
}

/// Estado pequeño y comprobable de la interfaz.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TuiApp {
    selected: usize,
    status: String,
    route_decision_json: Option<String>,
    route_summary: Option<String>,
    execution: ExecutionPanelState,
    interaction: TuiInteractionState,
}

impl TuiApp {
    /// Abre en catálogo sin mutar estado externo.
    pub fn new() -> Self {
        Self {
            selected: 0,
            status: "sólo se guarda tras confirmación".to_string(),
            route_decision_json: None,
            route_summary: None,
            execution: ExecutionPanelState::default(),
            interaction: TuiInteractionState::default(),
        }
    }

    /// Vista activa.
    pub const fn view(&self) -> TuiView {
        TuiView::ALL[self.selected]
    }

    /// Siguiente vista, con vuelta al principio.
    pub fn next_view(&mut self) {
        self.selected = (self.selected + 1) % TuiView::ALL.len();
    }

    /// Evalúa un sobre con la misma función pura que CLI y MCP.
    ///
    /// # Errors
    ///
    /// Devuelve el mismo [`ApiError`] versionado que `batuta route`.
    pub fn explain_route(
        &mut self,
        service: &ApplicationService,
        input: &str,
    ) -> Result<(), ApiError> {
        let decision = route_json(service, input)?;
        let value = serde_json::from_str::<serde_json::Value>(&decision).ok();
        self.route_summary = value.as_ref().map(|value| {
            format!(
                "Ruta: {} · investigado: {} · override: {} · efectivo: {} · cobertura: {}% · verificada: {} · evidencia: {} · política: {}",
                value["data"]["route"].as_str().unwrap_or("?"),
                value["data"]["researched_score"],
                if value["data"]["manual_override"].is_null() { "sin override" } else { "auditado" },
                value["data"]["effective_score"],
                value["data"]["coverage"],
                value["data"]["verified"],
                value["data"]["evidence_hash"].as_str().unwrap_or("?"),
                value["data"]["policy_hash"].as_str().unwrap_or("?")
            )
        });
        self.route_decision_json = Some(decision);
        Ok(())
    }

    /// Decisión JSON exacta explicada actualmente por la TUI.
    pub fn route_decision_json(&self) -> Option<&str> {
        self.route_decision_json.as_deref()
    }

    /// Lanza una actualización total bajo demanda y deja sólo una solicitud en staging.
    ///
    /// # Errors
    ///
    /// Devuelve un error si la solicitud no puede persistirse atómicamente.
    pub fn queue_research_update(&mut self, layout: &Layout) -> Result<String, String> {
        let request = queue_research_update(layout, &ResearchScope::All)?;
        self.status = format!("investigación en staging: {request}; activo sin cambios");
        Ok(request)
    }

    /// Texto equivalente al cuerpo visual, útil para accesibilidad y tests.
    pub fn snapshot(&self, _width: u16) -> String {
        let body = match self.view() {
            TuiView::Catalog => {
                "Harness · proveedor · modelo · alias\nImportar/confirmar rutas DSH · habilitar/deshabilitar"
            }
            TuiView::Profiles => {
                "Acción · esfuerzo · coste relativo · margen\nCestas · fallbacks · allow_any_eligible"
            }
            TuiView::Evidence => {
                "Puntaje investigado · override · efectivo · cobertura\nActualizar investigación crea staging; aplicar exige confirmación"
            }
            TuiView::Health => "Cooldown · latencia p95 · éxito reciente · procedencia · recibos",
            TuiView::Execution => {
                return format!(
                    "{}\n{}\n{}",
                    self.view().title(),
                    self.execution_snapshot(),
                    self.status
                );
            }
        };
        let route = self
            .route_summary
            .as_deref()
            .map_or(String::new(), |summary| format!("\n{summary}"));
        format!("{}\n{body}{route}\n{}", self.view().title(), self.status)
    }
}

impl Default for TuiApp {
    fn default() -> Self {
        Self::new()
    }
}

/// Ejecuta la TUI hasta `q` o `Esc` y restaura siempre el terminal.
///
/// # Errors
///
/// Devuelve un error si falla la inicialización, el dibujo o la restauración del terminal.
pub fn run_tui(
    layout: &Layout,
    service: &ApplicationService,
    route_input: Option<&str>,
) -> io::Result<()> {
    let mut app = TuiApp::new();
    if let Some(input) = route_input {
        app.explain_route(service, input)
            .map_err(io::Error::other)?;
    }
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableBracketedPaste)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let result = terminal::tui_loop(&mut terminal, app, layout);
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        DisableBracketedPaste,
        LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;
    result
}
