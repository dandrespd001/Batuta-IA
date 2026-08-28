//! Lo que impide llegar a tener un veredicto.
//!
//! Ninguno de estos es un canario rojo. Un canario rojo **es** una respuesta: el
//! proveedor contestó y lo que contestó no vale. Esto otro es no haber llegado a
//! preguntar, y confundir las dos cosas es cómo un informe acaba diciendo que
//! algo se hizo cuando no se hizo.

use std::fmt;
use std::path::PathBuf;

/// Por qué la orden no llegó a producir un recibo.
#[derive(Debug)]
#[non_exhaustive]
pub enum CliError {
    /// La orden no existe.
    UnknownCommand {
        /// Lo que se escribió.
        given: String,
        /// Las que hay.
        available: Vec<&'static str>,
    },
    /// Una bandera vino sin su valor.
    ///
    /// No se traga el siguiente argumento: `--provider --model dsh` es un error,
    /// no un proveedor llamado `--model`.
    MissingValue {
        /// La bandera.
        flag: String,
    },
    /// Una bandera obligatoria no vino.
    MissingFlag {
        /// La bandera.
        flag: &'static str,
    },
    /// Dos banderas que se contradicen.
    ///
    /// No se resuelve por preferencia ni por orden de aparición: elegir en
    /// silencio entre dos instrucciones incompatibles es la forma exacta en que
    /// se pidió un modelo y corrió otro.
    ContradictoryFlags {
        /// Una.
        one: &'static str,
        /// La otra.
        other: &'static str,
    },
    /// Una bandera que no se admite.
    UnknownFlag {
        /// Lo que se escribió.
        given: String,
        /// Las que hay.
        available: Vec<&'static str>,
    },
    /// Ningún manifiesto del directorio declara ese proveedor.
    ///
    /// El error **enumera los que sí hay** (R8): un `"provider not found"`
    /// obliga a ir a mirar el directorio a mano.
    UnknownProvider {
        /// Lo que se pidió.
        asked: String,
        /// Los que hay.
        available: Vec<String>,
    },
    /// El proveedor declara varios modelos y no se pidió ninguno.
    ///
    /// **No se elige en silencio.** Elegir en silencio es exactamente cómo se
    /// pidió un modelo tres veces y corrió otro las tres.
    AmbiguousModel {
        /// El proveedor.
        provider: String,
        /// Sus modelos.
        available: Vec<String>,
    },
    /// El modelo pedido no es de ese proveedor.
    UnknownModel {
        /// Lo que se pidió.
        asked: String,
        /// El proveedor.
        provider: String,
        /// Los suyos.
        available: Vec<String>,
    },
    /// Un argumento posicional obligatorio no vino.
    ///
    /// Distinto de `MissingFlag`: `enable`/`disable`/`effort` no llevan
    /// banderas, llevan posiciones, y una posición ausente merece su propio
    /// nombre en vez de fingir que era una bandera que faltó.
    MissingArgument {
        /// La orden.
        command: &'static str,
        /// Qué faltaba, en prosa: `<proveedor>/<modelo>` o `<nivel>`.
        argument: &'static str,
    },
    /// Sobró un argumento posicional.
    ///
    /// No se ignora ni se resuelve por posición: un argumento de más es tan
    /// ambiguo como una bandera que se contradice con otra, y la reacción es
    /// la misma —parar y decirlo, no adivinar cuál de los dos vale.
    UnexpectedArgument {
        /// La orden.
        command: &'static str,
        /// Lo que sobraba.
        given: String,
    },
    /// `<proveedor>/<modelo>` sin la barra.
    MalformedModelRef {
        /// Lo que se escribió.
        given: String,
    },
    /// El nivel de `effort` no es ninguno de los que admite `ReasoningEffort`.
    InvalidReasoningEffort {
        /// Causa: ya enumera los válidos (R8), lo hereda del vocabulario.
        source: batuta_contract::VocabularyError,
    },
    /// El proveedor de ese modelo no declara ningún mapa de esfuerzo.
    ///
    /// No se guarda un nivel que nunca se va a poder honrar: `effort` falla
    /// aquí en vez de dejar una política con una promesa vacía.
    EffortUnsupported {
        /// El proveedor.
        provider: String,
    },
    /// `<id>` no valida como [`batuta_contract::ProviderId`].
    InvalidProviderId {
        /// Causa: ya enumera la regla (R8), lo hereda del identificador.
        source: batuta_contract::IdentifierError,
    },
    /// `<id>` no valida como [`batuta_contract::ModelId`].
    InvalidModelId {
        /// Causa.
        source: batuta_contract::IdentifierError,
    },
    /// `<ruta>` no valida como [`batuta_contract::RouteModel`].
    InvalidRouteModel {
        /// Causa.
        source: batuta_contract::IdentifierError,
    },
    /// `nuevo-proveedor` sobre un id que ya tiene fichero en `providers/`.
    ///
    /// Nunca se sobrescribe un proveedor existente: quien quiera cambiar uno
    /// edita el fichero a mano, que es exactamente la tesis de §1.
    ProviderAlreadyExists {
        /// El id pedido.
        id: String,
        /// Dónde ya existía.
        path: PathBuf,
    },
    /// `nuevo-modelo` con un id que ese proveedor ya declara.
    DuplicateModelId {
        /// El proveedor.
        provider: String,
        /// El id repetido.
        id: String,
    },
    /// `quitar-modelo` sobre el único modelo de un proveedor.
    ///
    /// Se comprueba **antes** de tocar el texto: dejar que lo detecte un
    /// `NoModels` de `ManifestError` después de escribir hablaría de un fallo
    /// de esquema, no de la razón real, y además habría que deshacer la
    /// escritura. `disable` existe para esto — apagar no borra nada.
    CannotRemoveLastModel {
        /// El proveedor.
        provider: String,
    },
    /// El modelo que ya se localizó en el manifiesto no se encontró al
    /// escanear el texto para quitarlo.
    ///
    /// No debería poder pasar —si el modelo está en el manifiesto que acaba
    /// de parsear, su bloque `[[models]]` está en ese mismo texto—, pero si
    /// alguna forma de TOML que el escáner de texto no reconoce y el
    /// analizador sí lo produce algún día, esto lo dice en vez de dejar que
    /// un `panic!` tumbe el proceso.
    ModelBlockNotFound {
        /// El proveedor.
        provider: String,
        /// El modelo.
        model: String,
    },
    /// Un manifiesto del directorio no se pudo cargar.
    ///
    /// Un manifiesto irresoluble falla **al cargar**, no a mitad de una corrida
    /// (R1): el directorio entero se lee antes de tocar nada.
    ///
    /// La causa va en caja: un `ManifestError` lleva fichero, línea y columna, y
    /// sin la caja cada `Ok` de cada función que pueda devolver este error
    /// pagaría ese tamaño. El error raro no debe encarecer el camino común.
    Manifest {
        /// Causa.
        source: Box<batuta_manifest::ManifestError>,
    },
    /// La corrida no se pudo llevar a cabo.
    Exec {
        /// Causa.
        source: Box<batuta_exec::ExecError>,
    },
    /// La política no se pudo leer o escribir.
    ///
    /// Un fichero de política ausente **no** cae aquí: la primera vez que se
    /// consulta el panel no hay ninguno, y eso es el estado inicial, no un
    /// error. Esto es para cuando el fichero está y está roto.
    Policy {
        /// Causa.
        source: Box<batuta_policy::PoliticaError>,
    },
    /// El almacén de recibos no se pudo consultar.
    ///
    /// Un recibo suelto ilegible tampoco cae aquí —eso lo cuenta el propio
    /// panel, aparte—: esto es cuando el directorio entero no se pudo listar.
    Store {
        /// Causa.
        source: Box<batuta_store::StoreError>,
    },
    /// El disco no cooperó.
    Io {
        /// Qué se intentaba.
        path: PathBuf,
        /// Causa.
        source: std::io::Error,
    },
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownCommand { given, available } => write!(
                f,
                "`{given}` no es una orden de batuta; las que hay: {}",
                available.join(", ")
            ),
            Self::MissingValue { flag } => {
                write!(f, "`{flag}` necesita un valor y vino sola")
            }
            Self::MissingFlag { flag } => write!(f, "falta `{flag}`"),
            Self::ContradictoryFlags { one, other } => {
                write!(f, "`{one}` y `{other}` se contradicen: elige una")
            }
            Self::UnknownFlag { given, available } => write!(
                f,
                "`{given}` no es una bandera de esta orden; las que hay: {}",
                available.join(", ")
            ),
            Self::UnknownProvider { asked, available } => write!(
                f,
                "no hay ningún proveedor `{asked}`; los que hay: {}",
                available.join(", ")
            ),
            Self::AmbiguousModel {
                provider,
                available,
            } => write!(
                f,
                "`{provider}` declara varios modelos: elige uno con `--model` entre {}",
                available.join(", ")
            ),
            Self::UnknownModel {
                asked,
                provider,
                available,
            } => write!(
                f,
                "`{provider}` no declara ningún modelo `{asked}`; los suyos: {}",
                available.join(", ")
            ),
            Self::MissingArgument { command, argument } => {
                write!(f, "`{command}` necesita {argument} y no vino")
            }
            Self::UnexpectedArgument { command, given } => {
                write!(f, "`{command}` no esperaba `{given}`: sobra")
            }
            Self::MalformedModelRef { given } => write!(
                f,
                "`{given}` no tiene la forma `<proveedor>/<modelo>` (falta la barra)"
            ),
            Self::InvalidReasoningEffort { source } => write!(f, "{source}"),
            Self::EffortUnsupported { provider } => write!(
                f,
                "`{provider}` no declara ningún mapa de esfuerzo: pedirle un nivel no se puede honrar"
            ),
            Self::InvalidProviderId { source }
            | Self::InvalidModelId { source }
            | Self::InvalidRouteModel { source } => write!(f, "{source}"),
            Self::ProviderAlreadyExists { id, path } => write!(
                f,
                "ya existe un proveedor `{id}` en {}: nuevo-proveedor no sobrescribe nunca uno existente",
                path.display()
            ),
            Self::DuplicateModelId { provider, id } => write!(
                f,
                "`{provider}` ya declara un modelo `{id}`: nuevo-modelo no admite ids repetidos"
            ),
            Self::CannotRemoveLastModel { provider } => write!(
                f,
                "`{provider}` sólo declara un modelo: quitarlo lo dejaría sin ninguno; usa `disable` en vez de borrar"
            ),
            Self::ModelBlockNotFound { provider, model } => write!(
                f,
                "`{provider}/{model}` está en el manifiesto pero su bloque no se encontró al escanear el texto (esto es un fallo interno de batuta, no del manifiesto)"
            ),
            Self::Manifest { source } => write!(f, "{source}"),
            Self::Exec { source } => write!(f, "{source}"),
            Self::Policy { source } => write!(f, "{source}"),
            Self::Store { source } => write!(f, "{source}"),
            Self::Io { path, source } => write!(f, "{}: {source}", path.display()),
        }
    }
}

impl std::error::Error for CliError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidReasoningEffort { source } => Some(source),
            Self::InvalidProviderId { source }
            | Self::InvalidModelId { source }
            | Self::InvalidRouteModel { source } => Some(source),
            Self::Manifest { source } => Some(source),
            Self::Exec { source } => Some(source),
            Self::Policy { source } => Some(source),
            Self::Store { source } => Some(source),
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}
