//! El recibo: qué se pidió, qué se ejecutó de verdad, y qué salió.
//!
//! Es el artefacto de la Fase 3 y la respuesta directa a `harness.py:454` del
//! orquestador viejo, que reportaba `"Harness worker failed with exit 1"` y
//! descartaba stdout y stderr del hijo. Esa ceguera costó días de diagnóstico.
//!
//! La regla que lo ordena todo: **el recibo anota lo observado, no lo pedido.**

use std::path::PathBuf;
use std::time::Duration;

use crate::verdict::Verdict;

/// Un fichero que batuta materializó para la corrida, con su contenido.
///
/// Va en el recibo porque **el modelo no viaja en `argv`**: viaja en un
/// documento que batuta escribe. Un recibo sin esto no permite reproducir la
/// corrida ni explicar por qué corrió lo que corrió.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterializedFile {
    _path: PathBuf,
    _content: String,
}

/// Lo que la máquina anotó sobre la corrida, leído de su registro.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedProvenance {
    _provider: String,
    _model: String,
    _session_ids: Vec<String>,
    _tool_calls: Vec<(String, u32)>,
    _sandbox_mode: Option<String>,
    _permission_preset: Option<String>,
}

impl MaterializedFile {
    /// Un fichero de corrida, con su contenido tal como se escribió.
    pub fn new(path: PathBuf, content: String) -> Self {
        Self {
            _path: path,
            _content: content,
        }
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
            _provider: provider,
            _model: model,
            _session_ids: session_ids,
            _tool_calls: tool_calls,
            _sandbox_mode: sandbox_mode,
            _permission_preset: permission_preset,
        }
    }

    /// El proveedor que corrió de verdad.
    pub fn provider(&self) -> &str {
        todo!()
    }

    /// El modelo que corrió de verdad.
    pub fn model(&self) -> &str {
        todo!()
    }

    /// Los ids de sesión, en orden de intento.
    ///
    /// **El índice de los intentos es el recibo, no dsh**: `SessionHeader` es
    /// inmutable y el enlace padre-hijo sólo lo escribe `fork()`, que el modo
    /// headless no expone.
    pub fn session_ids(&self) -> &[String] {
        todo!()
    }

    /// Herramientas realmente invocadas, con su recuento.
    pub fn tool_calls(&self) -> &[(String, u32)] {
        todo!()
    }

    /// La jaula que el proveedor anotó al crear la sesión, si la anotó.
    ///
    /// batuta no construye el confinamiento: lo declara y **comprueba que se
    /// aplicó**. Que conste aquí es lo que convierte esa declaración en un hecho.
    pub fn sandbox_mode(&self) -> Option<&str> {
        todo!()
    }
}

/// El recibo de una corrida. Existe, luego es coherente.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Receipt {
    _provider: String,
    _model_requested: String,
    _manifest: PathBuf,
    _manifest_sha256: String,
    _argv: Vec<String>,
    _cwd: PathBuf,
    _env_names: Vec<String>,
    _runtime_files: Vec<MaterializedFile>,
    _exit_code: Option<i32>,
    _stdout: String,
    _stderr: String,
    _duration: Duration,
    _observed: Option<ObservedProvenance>,
    _verdict: Verdict,
}

impl Receipt {
    /// El `argv` **real** con el que se lanzó el proceso, no el del manifiesto.
    ///
    /// Entre uno y otro hay sustituciones, y el que importa para reproducir es
    /// éste.
    pub fn argv(&self) -> &[String] {
        todo!()
    }

    /// Código de salida. `None` cuando lo mató una señal, que no es lo mismo
    /// que salir con error.
    pub fn exit_code(&self) -> Option<i32> {
        todo!()
    }

    /// stderr **íntegro**, siempre, aunque el proceso saliera con cero.
    ///
    /// Tres causas locales distintas dieron el mismo 0-bytes en stdout el
    /// 2026-08-25, y el error literal estaba en stderr las tres veces.
    pub fn stderr(&self) -> &str {
        todo!()
    }

    /// Los **nombres** de las variables de entorno que se pasaron.
    ///
    /// Nunca sus valores: un recibo es un documento que se lee y se comparte, y
    /// R10 dice que la configuración lleva referencias, jamás secretos.
    pub fn env_names(&self) -> &[String] {
        todo!()
    }

    /// La procedencia observada, si se pudo leer.
    ///
    /// `None` **no** se rellena con lo pedido: el veredicto es rojo con motivo
    /// [`RedReason::ProvenanceUnreadable`].
    ///
    /// [`RedReason::ProvenanceUnreadable`]: crate::verdict::RedReason::ProvenanceUnreadable
    pub fn observed(&self) -> Option<&ObservedProvenance> {
        todo!()
    }

    /// El veredicto, con su motivo si es rojo.
    pub fn verdict(&self) -> &Verdict {
        todo!()
    }

    /// Serializa el recibo a JSON estable.
    ///
    /// # Errors
    ///
    /// Si la serialización falla.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        todo!()
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
    /// El modelo que batuta pidió.
    pub model_requested: String,
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
    // Fase roja: el cuerpo aún no consume los hechos, pero la implementación sí
    // los consume —de eso se trata: sellar es tomar posesión de ellos—. El aviso
    // desaparece con el cuerpo; el `allow` no debe sobrevivirle.
    #[allow(clippy::needless_pass_by_value)]
    pub fn seal(_facts: RunFacts) -> Self {
        todo!("las reglas de derivación las fija tests/recibo.rs, en su orden")
    }
}
