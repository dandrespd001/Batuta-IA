// generado: deepseek-v4-flash - revisado: Arquitecto
//! Por qué una corrida no se pudo ni intentar.
//!
//! Ojo con la distinción, porque es la que separa este error del veredicto del
//! recibo: **aquí sólo viven los fallos que impiden ejecutar**. Una corrida que
//! arrancó y salió mal no es un `ExecError`: es un recibo en rojo con su motivo.
//! Confundirlos haría que un modelo que contesta mal y un manifiesto roto se
//! parecieran, y son cosas distintas para quien lee.

use std::fmt;
use std::path::PathBuf;

/// La corrida no llegó a empezar.
#[derive(Debug)]
pub enum ExecError {
    /// Una llave `{...}` que no es incorporada ni declarada.
    UnknownPlaceholder {
        /// Dónde apareció: `invoke.argv` o la ruta del documento.
        field: String,
        /// La llave que no se reconoce.
        placeholder: String,
        /// Todas las admitidas (R8).
        expected: Vec<String>,
    },
    /// El manifiesto usa una llave incorporada que este encargo no puede llenar.
    ///
    /// Sólo `{route_provider}` puede faltar: es la única incorporada opcional.
    /// Sustituirla por vacío sería lo cómodo y es lo peor de las tres salidas —la
    /// cadena vacía viaja hasta el `argv` de un proceso real y nadie la ve—, así
    /// que se para aquí. Un manifiesto que pide la ruta del proveedor y un modelo
    /// que no la declara es un emparejamiento incoherente, y decirlo cuesta menos
    /// que descubrirlo en una corrida.
    MissingBuiltin {
        /// Dónde aparecía la llave.
        field: String,
        /// La llave que no se puede llenar.
        placeholder: String,
    },
    /// Un fichero de corrida caería **dentro del worktree**.
    ///
    /// Es la comprobación que `batuta-manifest` no podía hacer, porque `parse()`
    /// es puro y el worktree no existe cuando se carga un manifiesto. Aquí sí se
    /// conocen las dos rutas, y por eso vive aquí. Se rechaza **antes** de
    /// ejecutar: un fichero de configuración de batuta dentro del árbol del
    /// encargo aparecería en el diff como si fuera trabajo del modelo.
    RuntimeFileInsideWorktree {
        /// La ruta que caería dentro.
        path: PathBuf,
        /// El worktree que invade.
        worktree: PathBuf,
    },
    /// No se pudo escribir un fichero de corrida.
    Materialize {
        /// Ruta implicada.
        path: PathBuf,
        /// Causa.
        source: std::io::Error,
    },
    /// El proceso no se pudo lanzar.
    Spawn {
        /// El programa que se intentó.
        program: PathBuf,
        /// Causa.
        source: std::io::Error,
    },
}

/// El `Display` es un `match` lineal, una variante por mensaje, redactado desde
/// los doc-comments de cada variante: es lo que llega a quien lee un recibo en
/// rojo, y no hay test que lo fije todavía salvo el de `UnknownPlaceholder`,
/// que exige que aparezca la llave ofensiva.
impl fmt::Display for ExecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownPlaceholder {
                field,
                placeholder,
                expected,
            } => {
                write!(
                    f,
                    "llave de sustitución desconocida `{{{placeholder}}}` en `{field}`; admitidas: "
                )?;
                for (indice, admitida) in expected.iter().enumerate() {
                    if indice > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{{{admitida}}}")?;
                }
                Ok(())
            }
            Self::MissingBuiltin { field, placeholder } => write!(
                f,
                "el manifiesto usa `{{{placeholder}}}` en `{field}` y este encargo no la trae; \
                 no se sustituye por vacío, porque el vacío llegaría al proceso sin que nadie \
                 lo vea"
            ),
            Self::RuntimeFileInsideWorktree { path, worktree } => write!(
                f,
                "el fichero de corrida `{}` caería dentro del worktree `{}`; se rechaza antes de escribir",
                path.display(),
                worktree.display()
            ),
            Self::Materialize { path, source } => write!(
                f,
                "no se pudo escribir el fichero de corrida `{}`: {source}",
                path.display()
            ),
            Self::Spawn { program, source } => {
                write!(
                    f,
                    "no se pudo lanzar el programa `{}`: {source}",
                    program.display()
                )
            }
        }
    }
}

impl std::error::Error for ExecError {}
