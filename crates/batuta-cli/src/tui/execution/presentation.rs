//! Presentación de propuestas, previews y resultados del worker.

use super::TuiExecutionSection;
use crate::ApiErrorV2;
use crate::tui::TuiApp;

impl TuiApp {
    pub(in crate::tui) fn execution_snapshot(&self) -> String {
        let detail = match self.execution_section() {
            TuiExecutionSection::Profile => self.execution.profile_proposal.as_ref().map_or_else(
                || {
                    "formulario cerrado · validar · staging/diff · confirmar escribiendo proposal ID"
                        .to_string()
                },
                |proposal| {
                    format!(
                        "propuesta: {}\nbase: {}\nperfil: {}\ndiff:\n{}",
                        proposal.id,
                        proposal.expected_active_hash,
                        proposal.proposed_profile_hash,
                        proposal.diff
                    )
                },
            ),
            TuiExecutionSection::Grants => {
                "crear desde borrador · estado · revocación confirmada · historia preservada"
                    .to_string()
            }
            TuiExecutionSection::Runs => self.execution.run_preview.as_ref().map_or_else(
                || {
                    "formulario RunRequestV2 o JSON · preview sin reserva · confirmar escribiendo run ID"
                        .to_string()
                },
                |preview| {
                    format!(
                        "preview: {} · sin reserva\nmanifest: {}\nruta: {}\ngrant: {} ({})\npresupuesto: {:?}\ndeadline_ms: {}",
                        preview.run_id,
                        preview.manifest_hash,
                        preview.route,
                        preview.grant_id,
                        preview.grant_hash,
                        preview.budget,
                        preview.deadline_at_ms
                    )
                },
            ),
        };
        format!(
            "Perfil · Grants · Runs\n{}: {detail}{}",
            self.execution_section().title(),
            self.execution_input_snapshot()
        )
    }

    pub(in crate::tui) fn accept_worker_result(&mut self, result: Result<String, ApiErrorV2>) {
        self.status = match result {
            Ok(output) => format!("worker completó: {output}"),
            Err(error) => format!("worker falló: {error}"),
        };
    }
}
