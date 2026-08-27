//! Identificadores validados en la frontera.
//!
//! Todos son `String` por dentro y todos son tipos distintos por fuera. El
//! motivo no es purismo: un `ProviderId` acaba en un `argv`, un
//! [`CredentialName`] acaba en `systemd-creds decrypt --name=`, y un
//! [`RelativePath`] acaba decidiendo qué puede escribir un modelo externo. Si el
//! valor se valida al construirlo, no hay ningún punto posterior donde haya que
//! acordarse de validarlo.
//!
//! [`ModelId`] y [`RouteModel`] son deliberadamente tipos distintos: el primero
//! es el nombre canónico de batuta (`abacus-glm-5.3-flash`), el segundo es el
//! que entiende el proveedor (`ZAI GLM 5.3 Flash`, con espacios y mayúsculas).
//! Confundirlos es exactamente la clase de error que R10 paga con semanas de
//! credencial ausente.

use alloc::borrow::ToOwned;
use alloc::string::String;
use core::fmt;

/// Motivo concreto por el que un identificador no vale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentifierProblem {
    /// Cadena vacía.
    Empty,
    /// Más largo de lo admitido.
    TooLong {
        /// Longitud recibida, en bytes.
        len: usize,
        /// Longitud máxima admitida, en bytes.
        max: usize,
    },
    /// Un carácter que el vocabulario del identificador no admite.
    InvalidCharacter {
        /// El carácter ofensivo.
        character: char,
        /// Su posición en bytes desde el principio.
        position: usize,
    },
    /// El primer carácter no vale como inicio.
    InvalidStart {
        /// El carácter con el que empieza.
        character: char,
    },
    /// El último carácter no vale como final.
    InvalidEnd {
        /// El carácter con el que termina.
        character: char,
    },
    /// Una ruta absoluta donde se esperaba una relativa.
    AbsolutePath,
    /// Un componente `..` o `.`: se sale del árbol o lo enmascara.
    PathTraversal,
    /// Un componente vacío, por barra doble o barra final.
    EmptyComponent,
    /// Espacio en blanco al principio o al final.
    SurroundingWhitespace,
}

impl fmt::Display for IdentifierProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("empty"),
            Self::TooLong { len, max } => write!(f, "{len} bytes, max {max}"),
            Self::InvalidCharacter {
                character,
                position,
            } => write!(f, "invalid character {character:?} at byte {position}"),
            Self::InvalidStart { character } => write!(f, "invalid first character {character:?}"),
            Self::InvalidEnd { character } => write!(f, "invalid last character {character:?}"),
            Self::AbsolutePath => f.write_str("absolute path"),
            Self::PathTraversal => f.write_str("'.' or '..' component"),
            Self::EmptyComponent => f.write_str("empty path component"),
            Self::SurroundingWhitespace => f.write_str("leading or trailing whitespace"),
        }
    }
}

/// Identificador rechazado, con el valor recibido y la regla que incumple.
///
/// Misma disciplina que R8 en los vocabularios: el error no dice «inválido»,
/// dice qué llegó y qué se admitía.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentifierError {
    kind: &'static str,
    rule: &'static str,
    value: String,
    problem: IdentifierProblem,
}

impl IdentifierError {
    pub(crate) fn new(
        kind: &'static str,
        rule: &'static str,
        value: &str,
        problem: IdentifierProblem,
    ) -> Self {
        Self {
            kind,
            rule,
            value: value.to_owned(),
            problem,
        }
    }

    /// Clase de identificador que rechazó el valor, p. ej. `provider_id`.
    pub const fn kind(&self) -> &'static str {
        self.kind
    }

    /// Regla que el valor incumple, en prosa.
    pub const fn rule(&self) -> &'static str {
        self.rule
    }

    /// Valor recibido, literal.
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Motivo concreto del rechazo.
    pub const fn problem(&self) -> IdentifierProblem {
        self.problem
    }
}

impl fmt::Display for IdentifierError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid {}: '{}' ({}). rule: {}",
            self.kind, self.value, self.problem, self.rule
        )
    }
}

impl core::error::Error for IdentifierError {}

/// Declara un identificador validado en su constructor.
macro_rules! validated_id {
    (
        $(#[$meta:meta])*
        $name:ident = $kind:literal, max = $max:expr, rule = $rule:literal, check = $check:path
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            #[doc = concat!("Clase de este identificador: `", $kind, "`.")]
            pub const KIND: &'static str = $kind;

            #[doc = concat!("Regla que cumple todo `", $kind, "` construido.")]
            pub const RULE: &'static str = $rule;

            /// Longitud máxima admitida, en bytes.
            pub const MAX_LEN: usize = $max;

            /// Texto del identificador.
            pub fn as_str(&self) -> &str {
                &self.0
            }

            /// Consume el identificador y devuelve su texto.
            pub fn into_string(self) -> String {
                self.0
            }

            fn validar(value: &str) -> ::core::result::Result<(), $crate::ids::IdentifierError> {
                let problema = if value.is_empty() {
                    ::core::option::Option::Some($crate::ids::IdentifierProblem::Empty)
                } else if value.len() > Self::MAX_LEN {
                    ::core::option::Option::Some($crate::ids::IdentifierProblem::TooLong {
                        len: value.len(),
                        max: Self::MAX_LEN,
                    })
                } else {
                    $check(value).err()
                };

                match problema {
                    ::core::option::Option::Some(problema) => ::core::result::Result::Err(
                        $crate::ids::IdentifierError::new($kind, $rule, value, problema),
                    ),
                    ::core::option::Option::None => ::core::result::Result::Ok(()),
                }
            }
        }

        impl ::core::convert::AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }

        impl ::core::fmt::Display for $name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl ::core::str::FromStr for $name {
            type Err = $crate::ids::IdentifierError;

            fn from_str(value: &str) -> ::core::result::Result<Self, Self::Err> {
                Self::validar(value)?;
                ::core::result::Result::Ok(Self(::alloc::borrow::ToOwned::to_owned(value)))
            }
        }

        impl ::serde::Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> ::core::result::Result<S::Ok, S::Error>
            where
                S: ::serde::Serializer,
            {
                serializer.serialize_str(&self.0)
            }
        }

        impl<'de> ::serde::Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> ::core::result::Result<Self, D::Error>
            where
                D: ::serde::Deserializer<'de>,
            {
                struct Visitante;

                impl ::serde::de::Visitor<'_> for Visitante {
                    type Value = $name;

                    fn expecting(
                        &self,
                        f: &mut ::core::fmt::Formatter<'_>,
                    ) -> ::core::fmt::Result {
                        ::core::write!(f, "{} ({})", $kind, $rule)
                    }

                    fn visit_str<E>(self, value: &str) -> ::core::result::Result<$name, E>
                    where
                        E: ::serde::de::Error,
                    {
                        value.parse().map_err(::serde::de::Error::custom)
                    }
                }

                deserializer.deserialize_str(Visitante)
            }
        }
    };
}

/// Cuerpo de un slug: empieza por letra minúscula y termina en letra o dígito.
fn cuerpo_de_slug(value: &str) -> Result<(), IdentifierProblem> {
    let primero = value.chars().next().ok_or(IdentifierProblem::Empty)?;
    if !primero.is_ascii_lowercase() {
        return Err(IdentifierProblem::InvalidStart { character: primero });
    }
    let ultimo = value.chars().next_back().ok_or(IdentifierProblem::Empty)?;
    if !(ultimo.is_ascii_lowercase() || ultimo.is_ascii_digit()) {
        return Err(IdentifierProblem::InvalidEnd { character: ultimo });
    }
    Ok(())
}

/// Sólo `[a-z0-9_-]`, empezando por letra y terminando en letra o dígito.
fn es_slug(value: &str) -> Result<(), IdentifierProblem> {
    for (posicion, caracter) in value.char_indices() {
        let admitido = caracter.is_ascii_lowercase()
            || caracter.is_ascii_digit()
            || matches!(caracter, '-' | '_');
        if !admitido {
            return Err(IdentifierProblem::InvalidCharacter {
                character: caracter,
                position: posicion,
            });
        }
    }
    cuerpo_de_slug(value)
}

/// Un slug por componente, separados por `/`, admitiendo además `.` dentro del
/// componente para versiones como `abacus-glm-5.3-flash`.
fn es_model_id(value: &str) -> Result<(), IdentifierProblem> {
    for (posicion, caracter) in value.char_indices() {
        let admitido = caracter.is_ascii_lowercase()
            || caracter.is_ascii_digit()
            || matches!(caracter, '-' | '_' | '.' | '/');
        if !admitido {
            return Err(IdentifierProblem::InvalidCharacter {
                character: caracter,
                position: posicion,
            });
        }
    }
    for componente in value.split('/') {
        if componente.is_empty() {
            return Err(IdentifierProblem::EmptyComponent);
        }
        if componente == "." || componente == ".." {
            return Err(IdentifierProblem::PathTraversal);
        }
        cuerpo_de_slug(componente)?;
    }
    Ok(())
}

/// Sólo `[A-Z0-9_]`, empezando por letra mayúscula o `_`.
fn es_env_var(value: &str) -> Result<(), IdentifierProblem> {
    for (posicion, caracter) in value.char_indices() {
        let admitido =
            caracter.is_ascii_uppercase() || caracter.is_ascii_digit() || caracter == '_';
        if !admitido {
            return Err(IdentifierProblem::InvalidCharacter {
                character: caracter,
                position: posicion,
            });
        }
    }
    let primero = value.chars().next().ok_or(IdentifierProblem::Empty)?;
    if !(primero.is_ascii_uppercase() || primero == '_') {
        return Err(IdentifierProblem::InvalidStart { character: primero });
    }
    Ok(())
}

/// Una línea opaca: sin caracteres de control y sin espacio en los bordes.
fn es_linea_opaca(value: &str) -> Result<(), IdentifierProblem> {
    for (posicion, caracter) in value.char_indices() {
        if caracter.is_control() {
            return Err(IdentifierProblem::InvalidCharacter {
                character: caracter,
                position: posicion,
            });
        }
    }
    if value.trim() != value {
        return Err(IdentifierProblem::SurroundingWhitespace);
    }
    Ok(())
}

/// Ruta relativa contenida: ni absoluta, ni con `..`, ni con componentes vacíos.
fn es_ruta_relativa(value: &str) -> Result<(), IdentifierProblem> {
    if value.starts_with('/') {
        return Err(IdentifierProblem::AbsolutePath);
    }
    if value.starts_with('~') {
        return Err(IdentifierProblem::InvalidStart { character: '~' });
    }
    for (posicion, caracter) in value.char_indices() {
        if caracter.is_control() || caracter == '\\' {
            return Err(IdentifierProblem::InvalidCharacter {
                character: caracter,
                position: posicion,
            });
        }
    }
    if value.trim() != value {
        return Err(IdentifierProblem::SurroundingWhitespace);
    }
    for componente in value.split('/') {
        if componente.is_empty() {
            return Err(IdentifierProblem::EmptyComponent);
        }
        if componente == "." || componente == ".." {
            return Err(IdentifierProblem::PathTraversal);
        }
    }
    Ok(())
}

validated_id! {
    /// Identificador de un proveedor: `abacus`, `deepseek-flash`, `omniroute`.
    ///
    /// Es el nombre del fichero de manifiesto y la clave con la que la política
    /// habla del proveedor.
    ProviderId = "provider_id",
    max = 64,
    rule = "lowercase ascii letters, digits, '-' and '_'; starts with a letter, ends with a letter or digit",
    check = es_slug
}

validated_id! {
    /// Identificador canónico de un modelo dentro de batuta.
    ///
    /// Admite espacio de nombres por `/` (`ds/deepseek-v4-flash`) y punto de
    /// versión (`abacus-glm-5.3-flash`). **No** es el nombre que entiende el
    /// proveedor: para eso está [`RouteModel`].
    ModelId = "model_id",
    max = 128,
    rule = "lowercase slug components separated by '/', digits, '-', '_' and '.' allowed inside a component",
    check = es_model_id
}

validated_id! {
    /// Nombre del modelo **tal y como lo entiende el proveedor**.
    ///
    /// `ZAI GLM 5.3 Flash`, `Grok 4.6`, `MiniMax-M3`. Es opaco a propósito: se
    /// pasa literal al `argv` y batuta no lo interpreta. Lo único que se le
    /// exige es no traer control ni espacio en los bordes, porque eso rompería
    /// el `argv` en silencio.
    RouteModel = "route_model",
    max = 128,
    rule = "printable single line, no leading or trailing whitespace",
    check = es_linea_opaca
}

validated_id! {
    /// Nombre de una credencial sellada.
    ///
    /// R10 — **un secreto, un nombre, una vez**. Este valor sale del manifiesto
    /// del proveedor y de ningún otro sitio, y es el mismo que se le pasa a
    /// `systemd-creds decrypt --name=`. Por eso no admite `/` ni `.`: un nombre
    /// de credencial no es una ruta.
    CredentialName = "credential_name",
    max = 64,
    rule = "lowercase ascii letters, digits, '-' and '_'; never a path",
    check = es_slug
}

validated_id! {
    /// Nombre de una variable de entorno, para la allowlist de R5.
    EnvVarName = "env_var_name",
    max = 64,
    rule = "uppercase ascii letters, digits and '_'; starts with a letter or '_'",
    check = es_env_var
}

validated_id! {
    /// Identificador de un perfil de gates: `chunsa-determinism`.
    ///
    /// No es vocabulario cerrado: los perfiles se leen de configuración, así que
    /// batuta valida la forma y deja la existencia para quien los cargue.
    GateProfileId = "gate_profile_id",
    max = 64,
    rule = "lowercase ascii letters, digits, '-' and '_'; starts with a letter, ends with a letter or digit",
    check = es_slug
}

validated_id! {
    /// Ruta relativa contenida dentro del árbol de trabajo.
    ///
    /// R5 — contención determinista **por nombre**, sin clasificador. Una
    /// `allowed_write_paths` que admitiera `..` o rutas absolutas no sería una
    /// allowlist, sería un adorno.
    RelativePath = "relative_path",
    max = 4096,
    rule = "relative, '/'-separated, no empty, '.' or '..' components, no backslash",
    check = es_ruta_relativa
}

/// Versión del esquema de un documento de batuta (manifiesto, `TaskSpec`, recibo).
///
/// R1 — un documento con una versión que batuta no conoce **falla al cargar**,
/// no al ejecutar.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(transparent)]
pub struct SchemaVersion(u16);

impl SchemaVersion {
    /// La versión que batuta escribe hoy.
    pub const CURRENT: Self = Self(1);

    /// Todas las versiones que batuta sabe leer.
    pub const SUPPORTED: &'static [u16] = &[1];

    /// Construye una versión cualquiera, sin comprobar que se admita.
    pub const fn new(version: u16) -> Self {
        Self(version)
    }

    /// El número.
    pub const fn get(self) -> u16 {
        self.0
    }

    /// ¿Sabe batuta leer esta versión?
    pub fn is_supported(self) -> bool {
        Self::SUPPORTED.contains(&self.0)
    }

    /// Exige que la versión se admita.
    ///
    /// # Errors
    ///
    /// [`SchemaVersionError`] si la versión no está en [`SchemaVersion::SUPPORTED`],
    /// con la lista de las que sí.
    pub fn require_supported(self) -> Result<Self, SchemaVersionError> {
        if self.is_supported() {
            Ok(self)
        } else {
            Err(SchemaVersionError {
                found: self.0,
                supported: Self::SUPPORTED,
            })
        }
    }
}

impl fmt::Display for SchemaVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Versión de esquema que batuta no sabe leer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaVersionError {
    found: u16,
    supported: &'static [u16],
}

impl SchemaVersionError {
    /// Versión encontrada en el documento.
    pub const fn found(&self) -> u16 {
        self.found
    }

    /// Versiones que sí se admiten.
    pub const fn supported(&self) -> &'static [u16] {
        self.supported
    }
}

impl fmt::Display for SchemaVersionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unsupported schema_version: {}. supported: ", self.found)?;
        for (indice, version) in self.supported.iter().enumerate() {
            if indice > 0 {
                f.write_str(", ")?;
            }
            write!(f, "{version}")?;
        }
        Ok(())
    }
}

impl core::error::Error for SchemaVersionError {}
