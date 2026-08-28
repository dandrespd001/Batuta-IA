// generado: deepseek-v4-flash - revisado: Arquitecto
//! El veredicto de una corrida, y **por qué**.
//!
//! Un veredicto sin motivo nombrado es la mitad del fallo que R4 paga: el
//! orquestador viejo reportaba `"Harness worker failed with exit 1"` y tiraba
//! stdout y stderr del hijo, así que tres causas locales distintas —un binario
//! movido, una bandera ausente y la web autodenegada— daban el mismo mensaje.
//!
//! Nótese que esto **no** es un vocabulario cerrado de los de `batuta-contract`,
//! y la distinción no es descuido. Aquellos existen para valores que llegan de
//! fuera, donde un valor malo necesita un error que enumere los válidos (R8). Un
//! veredicto lo produce batuta y no lo parsea de nadie: nunca hay que rechazar
//! un veredicto ajeno.

use serde::Serialize;

/// Qué concluyó batuta sobre una corrida.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Verdict {
    /// La corrida hizo lo que decía, y consta.
    Green,
    /// La corrida no vale, y aquí está el motivo.
    Red(RedReason),
}

/// Por qué un recibo sale en rojo.
///
/// Son los cinco sitios donde una corrida puede fallar, más los dos que sólo se
/// ven después. Cada uno tiene mensaje propio: «falló» no es un diagnóstico.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum RedReason {
    /// Ninguna ruta de `resolve` dio un ejecutable. Es R1 llegando tarde.
    ExecutableUnresolved,
    /// El binario no es el que el manifiesto fijó (R11).
    DigestMismatch {
        /// Lo que decía el manifiesto.
        expected: String,
        /// Lo que había en disco.
        found: String,
    },
    /// El proceso salió mal. El código va aparte porque `None` (matado por
    /// señal) y `Some(1)` son cosas distintas.
    ProcessFailed {
        /// Código de salida, si lo hubo.
        exit_code: Option<i32>,
    },
    /// El canario no devolvió su token. **Observacional**: se compara con el
    /// token que se generó, no se busca una subcadena en un juicio propio (R3).
    TokenMissing,
    /// La procedencia no se pudo leer.
    ///
    /// No se rellena con lo pedido. «No pude leerlo» y «no pasó nada» son cosas
    /// distintas, y confundirlas es exactamente cómo un recibo empieza a mentir.
    ProvenanceUnreadable {
        /// Qué impidió leerla.
        detail: String,
    },
    /// Corrió un modelo distinto del pedido.
    ///
    /// El fallo que lo paga: se pidió `deepseek-v4-flash` tres veces y corrió
    /// otro las tres, porque el modelo lo decidía un fichero que batuta no
    /// controlaba.
    ProvenanceMismatch {
        /// Lo que batuta pidió.
        requested: String,
        /// Lo que la máquina anotó.
        observed: String,
    },
    /// Se usó una herramienta que el encargo no declaraba.
    ///
    /// Las herramientas del proveedor no se apagan, se observan: el registro
    /// anota cada llamada. Un encargo sin `web_research` cuyo registro muestra
    /// llamadas web es rojo, no un aviso.
    UndeclaredToolUse {
        /// Las herramientas usadas y no declaradas.
        tools: Vec<String>,
    },
    /// El diff toca rutas fuera de la allowlist.
    ///
    /// El sandbox del proveedor confina al worktree entero; la allowlist es más
    /// fina y el proveedor no la conoce. Sólo se puede verificar sobre el
    /// resultado, y por eso este motivo existe.
    ScopeViolation {
        /// Las rutas que sobran.
        paths: Vec<String>,
    },
}

impl Verdict {
    /// ¿Es verde?
    pub const fn is_green(&self) -> bool {
        matches!(self, Self::Green)
    }

    /// El motivo, si es rojo.
    pub const fn reason(&self) -> Option<&RedReason> {
        match self {
            Self::Green => None,
            Self::Red(reason) => Some(reason),
        }
    }
}

impl core::fmt::Display for RedReason {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ExecutableUnresolved => f.write_str("no se pudo resolver el ejecutable"),
            Self::DigestMismatch { expected, found } => write!(
                f,
                "el binario no es el del manifiesto: se esperaba {expected}, había {found}"
            ),
            Self::ProcessFailed {
                exit_code: Some(code),
            } => write!(f, "el proceso falló con código {code}"),
            Self::ProcessFailed { exit_code: None } => {
                f.write_str("el proceso fue matado por una señal")
            }
            Self::TokenMissing => f.write_str("el canario no devolvió su token"),
            Self::ProvenanceUnreadable { detail } => {
                write!(f, "no se pudo leer la procedencia: {detail}")
            }
            Self::ProvenanceMismatch {
                requested,
                observed,
            } => {
                write!(f, "corrió {observed}, y se había pedido {requested}")
            }
            Self::UndeclaredToolUse { tools } => write!(
                f,
                "se usaron herramientas no declaradas: {}",
                tools.join(", ")
            ),
            Self::ScopeViolation { paths } => write!(
                f,
                "el diff toca rutas fuera de la allowlist: {}",
                paths.join(", ")
            ),
        }
    }
}
