//! Renderizado y traducción de eventos del terminal.

use std::io;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout as TuiLayout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Tabs};

use super::{TuiApp, TuiExecutionSection, TuiExecutionWorker, TuiInputAction, TuiView};
use crate::Layout;

pub(super) fn tui_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    mut app: TuiApp,
    layout: &Layout,
) -> io::Result<()> {
    let worker = TuiExecutionWorker::for_layout(layout.clone());
    loop {
        if let Some(result) = worker.poll() {
            app.accept_worker_result(result);
        }
        draw(terminal, &app)?;
        if event::poll(Duration::from_millis(250))? {
            let input = event::read()?;
            if handle_event(&mut app, layout, &worker, input) {
                return Ok(());
            }
        }
    }
}

fn draw(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &TuiApp) -> io::Result<()> {
    terminal.draw(|frame| {
        let areas = TuiLayout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(5),
                Constraint::Length(3),
            ])
            .split(frame.area());
        let titles = TuiView::ALL
            .iter()
            .map(|view| Line::from(view.title()))
            .collect::<Vec<_>>();
        frame.render_widget(
            Tabs::new(titles)
                .select(app.selected)
                .block(Block::default().borders(Borders::ALL).title("Batuta"))
                .highlight_style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
            areas[0],
        );
        frame.render_widget(
            Paragraph::new(app.snapshot(areas[1].width))
                .block(Block::default().borders(Borders::ALL)),
            areas[1],
        );
        frame.render_widget(
            Paragraph::new(controls(app)).block(Block::default().borders(Borders::ALL)),
            areas[2],
        );
    })?;
    Ok(())
}

fn handle_event(
    app: &mut TuiApp,
    layout: &Layout,
    worker: &TuiExecutionWorker,
    event: Event,
) -> bool {
    if let Event::Paste(value) = event {
        if app.execution_input_active() {
            app.append_execution_input(&value.replace(['\r', '\n'], " "));
        }
        return false;
    }
    let Event::Key(key) = event else {
        return false;
    };
    if key.kind == KeyEventKind::Release {
        return false;
    }
    if app.execution_input_active() {
        match key.code {
            KeyCode::Esc => app.cancel_execution_input(),
            KeyCode::Enter => {
                if let Err(error) = app.submit_execution_input(layout, worker, now_ms()) {
                    app.status = format!("entrada rechazada: {error}");
                }
            }
            KeyCode::Backspace => app.pop_execution_input(),
            KeyCode::Char(character) => app.append_execution_input(&character.to_string()),
            _ => {}
        }
        return false;
    }

    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => return true,
        KeyCode::Tab | KeyCode::Right => app.next_view(),
        KeyCode::Char('u') => {
            if let Err(error) = app.queue_research_update(layout) {
                app.status = format!("no se pudo crear staging: {error}");
            }
        }
        KeyCode::Char('s') if app.view() == TuiView::Execution => {
            app.next_execution_section();
        }
        KeyCode::Left => {
            app.selected = if app.selected == 0 {
                TuiView::ALL.len() - 1
            } else {
                app.selected - 1
            };
        }
        code if app.view() == TuiView::Execution => {
            if let Some(action) = execution_action(app.execution_section(), code)
                && let Err(error) = app.begin_execution_input(action)
            {
                app.status = format!("acción no disponible: {error}");
            }
        }
        _ => {}
    }
    false
}

fn execution_action(section: TuiExecutionSection, code: KeyCode) -> Option<TuiInputAction> {
    match (section, code) {
        (TuiExecutionSection::Profile, KeyCode::Char('f')) => Some(TuiInputAction::ProfileForm),
        (TuiExecutionSection::Profile, KeyCode::Char('j')) => Some(TuiInputAction::ProfileJson),
        (TuiExecutionSection::Profile, KeyCode::Char('a')) => Some(TuiInputAction::ProfileApply),
        (TuiExecutionSection::Grants, KeyCode::Char('n')) => Some(TuiInputAction::GrantCreate),
        (TuiExecutionSection::Grants, KeyCode::Char('v')) => Some(TuiInputAction::GrantStatus),
        (TuiExecutionSection::Grants, KeyCode::Char('x')) => Some(TuiInputAction::GrantRevoke),
        (TuiExecutionSection::Runs, KeyCode::Char('f')) => Some(TuiInputAction::RunForm),
        (TuiExecutionSection::Runs, KeyCode::Char('j')) => Some(TuiInputAction::RunJson),
        (TuiExecutionSection::Runs, KeyCode::Char('e')) => Some(TuiInputAction::RunExecute),
        (TuiExecutionSection::Runs, KeyCode::Char('v')) => Some(TuiInputAction::RunStatus),
        (TuiExecutionSection::Runs, KeyCode::Char('r')) => Some(TuiInputAction::RunResume),
        _ => None,
    }
}

fn controls(app: &TuiApp) -> &'static str {
    if app.execution_input_active() {
        return "Escriba/pegue · Enter: validar/siguiente · Backspace: borrar · Esc: cancelar";
    }
    if app.view() != TuiView::Execution {
        return "Tab/→: vista · ←: anterior · u: staging investigación · q/Esc: salir";
    }
    match app.execution_section() {
        TuiExecutionSection::Profile => {
            "s: sección · f: formulario · j: JSON · a: aplicar por ID · q/Esc: salir"
        }
        TuiExecutionSection::Grants => {
            "s: sección · n: crear JSON · v: estado · x: revocar por ID · q/Esc: salir"
        }
        TuiExecutionSection::Runs => {
            "s: sección · f: formulario · j: JSON · e: ejecutar · v: estado · r: resume"
        }
    }
}

fn now_ms() -> u64 {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    u64::try_from(millis).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cada_accion_execution_tiene_un_atajo_visible_en_su_seccion() {
        let cases = [
            (
                TuiExecutionSection::Profile,
                'f',
                TuiInputAction::ProfileForm,
            ),
            (
                TuiExecutionSection::Profile,
                'j',
                TuiInputAction::ProfileJson,
            ),
            (
                TuiExecutionSection::Profile,
                'a',
                TuiInputAction::ProfileApply,
            ),
            (
                TuiExecutionSection::Grants,
                'n',
                TuiInputAction::GrantCreate,
            ),
            (
                TuiExecutionSection::Grants,
                'v',
                TuiInputAction::GrantStatus,
            ),
            (
                TuiExecutionSection::Grants,
                'x',
                TuiInputAction::GrantRevoke,
            ),
            (TuiExecutionSection::Runs, 'f', TuiInputAction::RunForm),
            (TuiExecutionSection::Runs, 'j', TuiInputAction::RunJson),
            (TuiExecutionSection::Runs, 'e', TuiInputAction::RunExecute),
            (TuiExecutionSection::Runs, 'v', TuiInputAction::RunStatus),
            (TuiExecutionSection::Runs, 'r', TuiInputAction::RunResume),
        ];
        for (section, key, expected) in cases {
            assert_eq!(
                execution_action(section, KeyCode::Char(key)),
                Some(expected),
                "missing {key} in {section:?}"
            );
        }
    }
}
