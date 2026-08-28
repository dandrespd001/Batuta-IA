// generado: deepseek-v4-flash - revisado: Arquitecto
//! El recibo: qué se pidió, qué se ejecutó de verdad, y qué salió.
//!
//! Es el artefacto de la Fase 3 y la respuesta directa a `harness.py:454` del
//! orquestador viejo, que reportaba `"Harness worker failed with exit 1"` y
//! descartaba stdout y stderr del hijo. Esa ceguera costó días de diagnóstico.
//!
//! La regla que lo ordena todo: **el recibo anota lo observado, no lo pedido.**

use std::path::PathBuf;
use std::time::Duration;

use serde::Serialize;

use batuta_contract::ProvenanceSource;

use crate::verdict::{RedReason, Verdict};

/// Un fichero que batuta materializó para la corrida, con su contenido.
///
/// Va en el recibo porque **el modelo no viaja en `argv`**: viaja en un
/// documento que batuta escribe. Un recibo sin esto no permite reproducir la
/// corrida ni explicar por qué corrió lo que corrió.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MaterializedFile {
    path: PathBuf,
    content: String,
}

/// Lo que la máquina anotó sobre la corrida, leído de su registro.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ObservedProvenance {
    provider: String,
    model: String,
    session_ids: Vec<String>,
    tool_calls: Vec<(String, u32)>,
    sandbox_mode: Option<String>,
    permission_preset: Option<String>,
}

impl MaterializedFile {
    /// Un fichero de corrida, con su contenido tal como se escribió.
    pub fn new(path: PathBuf, content: String) -> Self {
        Self { path, content }
    }
}

impl ObservedProvenance {
    /// Lo leído del registro de la máquina.
    pub fn new(
        provider: String,
        model: String,
        session_ids: Vec<String>,
        tool_calls: Vec<(String, u32)>,
        sandbox_mode: Option<String>,
        permission_preset: Option<String>,
    ) -> Self {
        Self {
            provider,
            model,
            session_ids,
            tool_calls,
            sandbox_mode,
            permission_preset,
        }
    }

    /// El proveedor que corrió de verdad.
    pub fn provider(&self) -> &str {
        &self.provider
    }

    /// El modelo que corrió de verdad.
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Los ids de sesión, en orden de intento.
    ///
    /// **El índice de los intentos es el recibo, no `dsh`**: `SessionHeader` es
    /// inmutable y el enlace padre-hijo sólo lo escribe `fork()`, que el modo
    /// headless no expone.
    pub fn session_ids(&self) -> &[String] {
        &self.session_ids
    }

    /// Herramientas realmente invocadas, con su recuento.
    pub fn tool_calls(&self) -> &[(String, u32)] {
        &self.tool_calls
    }

    /// La jaula que el proveedor anotó al crear la sesión, si la anotó.
    ///
    /// batuta no construye el confinamiento: lo declara y **comprueba que se
    /// aplicó**. Que conste aquí es lo que convierte esa declaración en un hecho.
    pub fn sandbox_mode(&self) -> Option<&str> {
        self.sandbox_mode.as_deref()
    }
}

/// El recibo de una corrida. Existe, luego es coherente.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Receipt {
    provider: String,
    model_requested: String,
    /// El nombre con el que se le pidió al proveedor. Es el que el registro
    /// anota, y por tanto el único con el que se puede contrastar.
    route_model: String,
    manifest: PathBuf,
    manifest_sha256: String,
    argv: Vec<String>,
    cwd: PathBuf,
    env_names: Vec<String>,
    runtime_files: Vec<MaterializedFile>,
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
    /// Duración en milisegundos enteros: representación estable para el JSON,
    /// sin depender de cómo serialice cada plataforma `Duration`.
    duration_ms: u64,
    observed: Option<ObservedProvenance>,
    /// ¿Se pudo **comprobar** qué modelo corrió?
    ///
    /// No es lo mismo que el veredicto. Un proveedor sin registro legible puede
    /// dar verde —el transporte funciona— sin que nadie haya confirmado el
    /// modelo. Quien lea el recibo tiene que poder distinguirlo sin deducirlo.
    model_confirmed: bool,
    verdict: Verdict,
}

impl Receipt {
    /// El modelo que se pidió, con el identificador de batuta.
    pub fn model_requested(&self) -> &str {
        &self.model_requested
    }

    /// El mismo modelo, con el nombre que el proveedor entiende.
    ///
    /// Es el que se contrasta con el registro. El recibo lleva los dos porque
    /// quien lo lee necesita poder ir en las dos direcciones: del manifiesto al
    /// registro, y del registro al manifiesto.
    pub fn route_model(&self) -> &str {
        &self.route_model
    }

    /// El `argv` **real** con el que se lanzó el proceso, no el del manifiesto.
    ///
    /// Entre uno y otro hay sustituciones, y el que importa para reproducir es
    /// éste.
    pub fn argv(&self) -> &[String] {
        &self.argv
    }

    /// Código de salida. `None` cuando lo mató una señal, que no es lo mismo
    /// que salir con error.
    pub fn exit_code(&self) -> Option<i32> {
        self.exit_code
    }

    /// stderr **íntegro**, siempre, aunque el proceso saliera con cero.
    ///
    /// Tres causas locales distintas dieron el mismo 0-bytes en stdout el
    /// 2026-08-25, y el error literal estaba en stderr las tres veces.
    pub fn stderr(&self) -> &str {
        &self.stderr
    }

    /// Los **nombres** de las variables de entorno que se pasaron.
    ///
    /// Nunca sus valores: un recibo es un documento que se lee y se comparte, y
    /// R10 dice que la configuración lleva referencias, jamás secretos.
    pub fn env_names(&self) -> &[String] {
        &self.env_names
    }

    /// La procedencia observada, si se pudo leer.
    ///
    /// `None` **no** se rellena con lo pedido: el veredicto es rojo con motivo
    /// [`RedReason::ProvenanceUnreadable`].
    ///
    /// [`RedReason::ProvenanceUnreadable`]: crate::verdict::RedReason::ProvenanceUnreadable
    pub fn observed(&self) -> Option<&ObservedProvenance> {
        self.observed.as_ref()
    }

    /// ¿Quedó **comprobado** qué modelo corrió?
    ///
    /// `false` no implica rojo: un proveedor sin registro legible da verde
    /// cuando el transporte funciona. Lo que `false` dice es que ese verde
    /// significa «funcionó», no «corrió el modelo que pedí».
    pub fn model_confirmed(&self) -> bool {
        self.model_confirmed
    }

    /// El veredicto, con su motivo si es rojo.
    pub fn verdict(&self) -> &Verdict {
        &self.verdict
    }

    /// Serializa el recibo a JSON estable.
    ///
    /// La duración viaja como `duration_ms`, milisegundos enteros: estable,
    /// legible y sin depender de la representación de `Duration`.
    ///
    /// # Errors
    ///
    /// Si la serialización falla.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

/// Los hechos de una corrida, tal como los recogió `batuta-exec`.
///
/// Existe para que **nadie pueda pasar un veredicto**: el veredicto lo deriva
/// [`Receipt::seal`] de estos hechos. Un recibo verde no es algo que se declara,
/// es algo que se concluye — y ésa es la misma regla del proyecto aplicada a su
/// propio artefacto.
#[derive(Debug, Clone)]
pub struct RunFacts {
    /// Proveedor del manifiesto.
    pub provider: String,
    /// El modelo que batuta pidió, con **su** identificador.
    pub model_requested: String,
    /// El mismo modelo, con el nombre que **el proveedor** entiende.
    ///
    /// Son dos espacios de nombres distintos y el registro de sesión sólo conoce
    /// el segundo: el manifiesto de dsh llama `dsh-deepseek-v4-flash` a un modelo
    /// cuyo `route_model` es `deepseek-v4-flash`. Comparar el registro contra el
    /// identificador de batuta daría `ProvenanceMismatch` en **todas** las
    /// corridas, acusando al proveedor de correr otro modelo cuando corre el
    /// correcto.
    pub route_model: String,
    /// Si este proveedor deja registro legible, y por tanto si su procedencia se
    /// puede **comprobar** o sólo creer.
    ///
    /// Sin este dato el recibo no puede distinguir «no lo pude leer» de «este
    /// proveedor no lo ofrece», y esas dos cosas piden veredictos opuestos: la
    /// primera es rojo, la segunda es verde sin confirmación.
    pub provenance_source: ProvenanceSource,
    /// Manifiesto que gobernó la corrida.
    pub manifest: PathBuf,
    /// Su `sha256`, para poder reproducirla.
    pub manifest_sha256: String,
    /// El `argv` real, ya sustituido.
    pub argv: Vec<String>,
    /// Dónde se ejecutó.
    pub cwd: PathBuf,
    /// Nombres de las variables pasadas. Nunca valores.
    pub env_names: Vec<String>,
    /// Ficheros escritos para la corrida, con su contenido.
    pub runtime_files: Vec<MaterializedFile>,
    /// Código de salida; `None` si lo mató una señal.
    pub exit_code: Option<i32>,
    /// Salida estándar completa.
    pub stdout: String,
    /// Error estándar **íntegro**.
    pub stderr: String,
    /// Cuánto duró.
    pub duration: Duration,
    /// La procedencia leída, o el motivo por el que no se pudo leer.
    ///
    /// Es un `Result` y no un `Option` a propósito: un `None` invita a rellenar
    /// el hueco; un `Err` obliga a decir qué pasó.
    pub observed: Result<ObservedProvenance, String>,
    /// Token del canario, cuando la corrida es un canario.
    pub expected_token: Option<String>,
    /// Herramientas que el encargo declaraba. Las demás, si se usan, son rojas.
    pub declared_tools: Vec<String>,
    /// Rutas del diff fuera de la allowlist. Vacío si no hubo diff que mirar.
    pub scope_violations: Vec<String>,
}

impl Receipt {
    /// Sella un recibo **derivando** su veredicto de los hechos.
    ///
    /// El orden de comprobación importa y es el de la corrida: primero lo que
    /// impidió ejecutar, luego lo que salió mal al ejecutar, y sólo al final lo
    /// que se ve leyendo el registro. Un recibo que dijera «modelo equivocado»
    /// cuando en realidad el proceso ni arrancó estaría diagnosticando mal.
    ///
    /// # Panics
    ///
    /// Sólo si la duración de la corrida no cabe en `u64` milisegundos —unos
    /// 584 millones de años—, que no es una corrida sino un artefacto.
    pub fn seal(facts: RunFacts) -> Self {
        let verdict = Self::derive_verdict(&facts);

        let RunFacts {
            provider,
            model_requested,
            route_model,
            provenance_source,
            manifest,
            manifest_sha256,
            argv,
            cwd,
            env_names,
            runtime_files,
            exit_code,
            stdout,
            stderr,
            duration,
            observed,
            // Los tres siguientes ya los consumió `derive_verdict`: viven en el
            // veredicto, no en el recibo. Un recibo no repite la pregunta, lleva
            // la respuesta.
            expected_token: _,
            declared_tools: _,
            scope_violations: _,
        } = facts;

        // Confirmado sólo si había registro que leer, se leyó, **y nombra el
        // modelo que se pidió**. Un registro legible que nombra otro modelo no
        // confirma nada: confirma lo contrario.
        let model_confirmed = provenance_source == ProvenanceSource::SessionLog
            && observed
                .as_ref()
                .is_ok_and(|observada| observada.model() == route_model);

        Self {
            provider,
            model_requested,
            route_model,
            manifest,
            manifest_sha256,
            argv,
            cwd,
            env_names,
            runtime_files,
            exit_code,
            stdout,
            stderr,
            duration_ms: u64::try_from(duration.as_millis())
                .expect("la duración de una corrida cabe en u64 milisegundos"),
            observed: observed.ok(),
            model_confirmed,
            verdict,
        }
    }

    /// Deriva el veredicto de los hechos, en el orden de la corrida.
    ///
    /// Recibe los hechos enteros y no sus piezas: desmontarlos obligaba a
    /// mantener dos listas en paralelo, y cada campo nuevo del recibo era un
    /// argumento más aquí. `RunFacts` ya **es** la agrupación correcta.
    fn derive_verdict(facts: &RunFacts) -> Verdict {
        let RunFacts {
            exit_code,
            stdout,
            expected_token,
            observed,
            route_model,
            declared_tools,
            scope_violations,
            provenance_source,
            ..
        } = facts;
        let exit_code = *exit_code;
        let provenance_source = *provenance_source;
        let expected_token = expected_token.as_deref();
        // Primero lo que salió mal al ejecutar: un proceso que ni terminó bien
        // no puede diagnosticarse con lo que se ve leyendo el registro.
        if exit_code != Some(0) {
            return Verdict::Red(RedReason::ProcessFailed { exit_code });
        }

        // El canario es observacional: se compara con el token generado, nunca
        // se busca una subcadena en un juicio propio (R3).
        if let Some(token) = expected_token
            && !stdout.contains(token)
        {
            return Verdict::Red(RedReason::TokenMissing);
        }

        // Luego lo que se ve leyendo el registro, en el orden del recibo:
        // procedencia, herramientas, y al final el alcance, que sólo se puede
        // verificar sobre el resultado.
        // Un proveedor sin registro no puede fallar por no tenerlo. Y tampoco se
        // le puede comprobar el uso de herramientas: por eso `abacus` contiene
        // por bandera (`--disallowed-tools "*"`) lo que dsh deja observar. Cada
        // uno con lo que su transporte permite, y el recibo dice cuál es cuál.
        if provenance_source == ProvenanceSource::Declared {
            if !scope_violations.is_empty() {
                return Verdict::Red(RedReason::ScopeViolation {
                    paths: scope_violations.clone(),
                });
            }
            return Verdict::Green;
        }

        let observed = match observed {
            Ok(observed) => observed,
            Err(detail) => {
                return Verdict::Red(RedReason::ProvenanceUnreadable {
                    detail: detail.clone(),
                });
            }
        };

        // Los dos nombres del **mismo** espacio: el que se le pidió al proveedor
        // y el que el proveedor anotó. El identificador de batuta no entra aquí.
        if observed.model() != route_model {
            return Verdict::Red(RedReason::ProvenanceMismatch {
                requested: route_model.to_owned(),
                observed: observed.model().to_owned(),
            });
        }

        let undeclared = undeclared_tools(observed.tool_calls(), declared_tools);
        if !undeclared.is_empty() {
            return Verdict::Red(RedReason::UndeclaredToolUse { tools: undeclared });
        }

        if !scope_violations.is_empty() {
            return Verdict::Red(RedReason::ScopeViolation {
                paths: scope_violations.clone(),
            });
        }

        Verdict::Green
    }
}

/// Las herramientas usadas y ausentes del encargo, en orden de aparición y sin
/// repetir.
fn undeclared_tools(used: &[(String, u32)], declared: &[String]) -> Vec<String> {
    let mut tools: Vec<String> = Vec::new();
    for (name, _calls) in used {
        if !declared.contains(name) && !tools.contains(name) {
            tools.push(name.clone());
        }
    }
    tools
}
