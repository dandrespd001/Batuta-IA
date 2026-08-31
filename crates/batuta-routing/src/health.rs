use batuta_contract::RouteRef;
use serde::{Deserialize, Serialize};

/// Resultado observado de una única invocación.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthOutcomeV2 {
    /// Resultado exitoso conocido.
    KnownSuccess,
    /// Fallo conocido.
    KnownFailure,
    /// La llamada pudo producir efectos pero no dejó resultado durable.
    Ambiguous,
}

/// Muestra individual conservada en la ventana de salud.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HealthObservationV2 {
    /// Instante Unix UTC.
    pub at: u64,
    /// Resultado conservador.
    pub outcome: HealthOutcomeV2,
    /// Latencia observada o tiempo reservado en un ambiguo.
    pub latency_ms: u64,
}

/// Categoría inferida de la respuesta del harness, nunca de consultar saldo o claves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureCategory {
    /// Rate limit con plazo explícito.
    RateLimited {
        /// Segundos pedidos por el harness.
        retry_after_seconds: u64,
    },
    /// Rate limit sin plazo.
    RateLimitedUnknown,
    /// Cuota agotada.
    QuotaExhausted,
    /// Autenticación inválida o caducada.
    Authentication,
    /// El harness informó que no hay saldo.
    Balance,
    /// Fallo transitorio no clasificado.
    Transient,
    /// El único intento agotó su tiempo de pared.
    Timeout,
    /// Fallo observado que no admite retry implícito.
    Permanent,
}

/// Salud persistible de una ruta.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouteHealth {
    /// Últimas veinte observaciones, en orden de llegada.
    pub observations: Vec<HealthObservationV2>,
    /// Tasa reciente de éxito `0..1`.
    pub recent_success_rate: f64,
    /// Latencia p95 observada.
    pub latency_p95_ms: u64,
    /// Fallos consecutivos.
    pub consecutive_failures: u32,
    /// Fin del cooldown.
    pub cooldown_until: Option<u64>,
    /// Sondas consecutivas programadas.
    pub probe_attempts: u32,
    /// Requiere intervención dentro del harness.
    pub blocked_by_harness: bool,
    /// Última categoría.
    pub last_failure: Option<FailureCategory>,
}

impl RouteHealth {
    /// Estado inicial sin inventar un fallo.
    pub const fn healthy() -> Self {
        Self {
            observations: Vec::new(),
            recent_success_rate: 1.0,
            latency_p95_ms: 0,
            consecutive_failures: 0,
            cooldown_until: None,
            probe_attempts: 0,
            blocked_by_harness: false,
            last_failure: None,
        }
    }

    /// Añade una muestra, recorta la ventana y recalcula tasa y p95.
    pub fn record(&mut self, observation: HealthObservationV2) {
        self.observations.push(observation);
        if self.observations.len() > 20 {
            let overflow = self.observations.len() - 20;
            self.observations.drain(..overflow);
        }
        let available = self.observations.len();
        if available == 0 {
            self.recent_success_rate = 1.0;
            self.latency_p95_ms = 0;
            return;
        }
        let successes = self
            .observations
            .iter()
            .filter(|item| item.outcome == HealthOutcomeV2::KnownSuccess)
            .fold(0_u32, |count, _| count.saturating_add(1));
        let available_u32 = u32::try_from(available).unwrap_or(20);
        self.recent_success_rate = f64::from(successes) / f64::from(available_u32);
        let mut latencies = self
            .observations
            .iter()
            .map(|item| item.latency_ms)
            .collect::<Vec<_>>();
        latencies.sort_unstable();
        let rank = (95 * available).div_ceil(100);
        self.latency_p95_ms = latencies[rank.saturating_sub(1)];
        match self.observations.last().map(|item| item.outcome) {
            Some(HealthOutcomeV2::KnownSuccess) => self.consecutive_failures = 0,
            Some(HealthOutcomeV2::KnownFailure | HealthOutcomeV2::Ambiguous) => {
                self.consecutive_failures = self.consecutive_failures.saturating_add(1);
            }
            None => {}
        }
    }
}

/// Próximo paso tras un fallo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryAction {
    /// Reanudar la misma ruta en el instante indicado.
    RetrySameRoute {
        /// Instante UTC Unix.
        at: u64,
    },
    /// Sondear la misma ruta sin ejecutar trabajo real.
    ProbeSameRoute {
        /// Instante UTC Unix.
        at: u64,
    },
    /// Crear checkpoint y escoger fallback ahora.
    FallbackImmediately,
}

/// Resultado puro de aplicar un evento de salud.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HealthTransition {
    /// Nueva salud durable.
    pub health: RouteHealth,
    /// Acción de recuperación.
    pub action: RecoveryAction,
}

/// Clasifica el próximo paso sin hacer ninguna llamada ni consultar secretos.
pub fn record_failure(
    current: &RouteHealth,
    route: &RouteRef,
    category: FailureCategory,
    now: u64,
) -> HealthTransition {
    let mut health = current.clone();
    health.consecutive_failures = health.consecutive_failures.saturating_add(1);
    health.last_failure = Some(category);
    let action = match category {
        FailureCategory::RateLimited {
            retry_after_seconds,
        } if retry_after_seconds <= 300 => {
            let at = now.saturating_add(retry_after_seconds);
            health.cooldown_until = Some(at);
            RecoveryAction::RetrySameRoute { at }
        }
        FailureCategory::RateLimitedUnknown if route.provider() == "minimax" => {
            let wait = match health.probe_attempts {
                0 => 15 * 60,
                1 => 30 * 60,
                _ => 60 * 60,
            };
            health.probe_attempts = health.probe_attempts.saturating_add(1);
            let at = now.saturating_add(wait);
            health.cooldown_until = Some(at);
            RecoveryAction::ProbeSameRoute { at }
        }
        FailureCategory::Authentication | FailureCategory::Balance => {
            health.blocked_by_harness = true;
            health.cooldown_until = None;
            RecoveryAction::FallbackImmediately
        }
        FailureCategory::RateLimited { .. }
        | FailureCategory::RateLimitedUnknown
        | FailureCategory::QuotaExhausted
        | FailureCategory::Transient
        | FailureCategory::Timeout
        | FailureCategory::Permanent => RecoveryAction::FallbackImmediately,
    };
    HealthTransition { health, action }
}
