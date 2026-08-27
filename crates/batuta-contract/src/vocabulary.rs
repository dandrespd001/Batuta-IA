//! Maquinaria de los vocabularios cerrados (R8).
//!
//! R8 dice: «Vocabularios enumerables; los errores listan los valores válidos».
//! El fallo que la paga fue `unknown output_contract: 'patch'` sin decir que
//! valen `text`, `json`, `unified_diff`, `review`.
//!
//! Aquí eso deja de depender de la disciplina de quien escribe el error: la
//! macro [`closed_vocabulary!`] genera a la vez el enum, su lista de tokens y el
//! error que los enumera. No hay forma de declarar un vocabulario cerrado en
//! batuta sin obtener también su mensaje de error completo.

use alloc::borrow::ToOwned;
use alloc::string::String;
use core::fmt;

/// Propiedad común a todo vocabulario cerrado de batuta.
///
/// Permite escribir comprobaciones genéricas —la suite de R8 lo hace— en lugar
/// de repetirlas vocabulario a vocabulario.
pub trait ClosedVocabulary:
    Sized + Copy + Eq + fmt::Debug + fmt::Display + core::str::FromStr<Err = VocabularyError> + 'static
{
    /// Nombre del vocabulario tal y como aparece en manifiestos y `TaskSpec`.
    const NAME: &'static str;

    /// Todas las variantes, en orden de declaración.
    const ALL: &'static [Self];

    /// Token textual de esta variante.
    fn as_str(self) -> &'static str;

    /// Todos los tokens válidos, en orden de declaración.
    fn tokens() -> &'static [&'static str];
}

/// Valor fuera de un vocabulario cerrado.
///
/// Lleva siempre los tres datos que R8 exige: qué vocabulario, qué se recibió y
/// qué se admitía.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VocabularyError {
    vocabulary: &'static str,
    value: String,
    expected: &'static [&'static str],
}

impl VocabularyError {
    /// Construye el error a partir del valor recibido y del vocabulario entero.
    pub(crate) fn new(
        vocabulary: &'static str,
        value: &str,
        expected: &'static [&'static str],
    ) -> Self {
        Self {
            vocabulary,
            value: value.to_owned(),
            expected,
        }
    }

    /// Nombre del vocabulario que rechazó el valor.
    pub const fn vocabulary(&self) -> &'static str {
        self.vocabulary
    }

    /// Valor recibido, literal y sin recortar.
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Valores que sí se admitían.
    pub const fn expected(&self) -> &'static [&'static str] {
        self.expected
    }
}

impl fmt::Display for VocabularyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unknown {}: '{}'. valid values: {}",
            self.vocabulary,
            self.value,
            ValueList(self.expected)
        )
    }
}

impl core::error::Error for VocabularyError {}

/// Lista de tokens separada por comas, para errores y para `expecting` de serde.
pub(crate) struct ValueList(pub(crate) &'static [&'static str]);

impl fmt::Display for ValueList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (indice, token) in self.0.iter().enumerate() {
            if indice > 0 {
                f.write_str(", ")?;
            }
            f.write_str(token)?;
        }
        Ok(())
    }
}

/// Declara un vocabulario cerrado con todo lo que R8 exige.
///
/// Genera el enum, `NAME`, `ALL`, `as_str`, `tokens`, `Display`, `FromStr` con
/// [`VocabularyError`], la implementación de [`ClosedVocabulary`] y el par
/// serde. El orden de declaración es significativo: es el orden de `ALL` y el de
/// `Ord`, y hay vocabularios —`sensitivity`, `reasoning_effort`— donde ese orden
/// *es* la política.
macro_rules! closed_vocabulary {
    (
        $(#[$meta:meta])*
        $name:ident = $wire:literal {
            $(
                $(#[$variant_meta:meta])*
                $variant:ident = $token:literal
            ),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum $name {
            $(
                $(#[$variant_meta])*
                $variant,
            )+
        }

        impl $name {
            #[doc = concat!("Nombre del vocabulario: `", $wire, "`.")]
            pub const NAME: &'static str = $wire;

            /// Todas las variantes, en orden de declaración.
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];

            /// Token textual de esta variante.
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $token),+
                }
            }

            /// Todos los tokens válidos, en orden de declaración.
            pub const fn tokens() -> &'static [&'static str] {
                &[$($token),+]
            }
        }

        impl $crate::vocabulary::ClosedVocabulary for $name {
            const NAME: &'static str = $wire;
            const ALL: &'static [Self] = &[$(Self::$variant),+];

            fn as_str(self) -> &'static str {
                Self::as_str(self)
            }

            fn tokens() -> &'static [&'static str] {
                Self::tokens()
            }
        }

        impl ::core::fmt::Display for $name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl ::core::str::FromStr for $name {
            type Err = $crate::vocabulary::VocabularyError;

            fn from_str(value: &str) -> ::core::result::Result<Self, Self::Err> {
                match value {
                    $($token => ::core::result::Result::Ok(Self::$variant),)+
                    otro => ::core::result::Result::Err(
                        $crate::vocabulary::VocabularyError::new($wire, otro, Self::tokens()),
                    ),
                }
            }
        }

        impl ::serde::Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> ::core::result::Result<S::Ok, S::Error>
            where
                S: ::serde::Serializer,
            {
                serializer.serialize_str(self.as_str())
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
                        ::core::write!(
                            f,
                            "{} (uno de: {})",
                            $wire,
                            $crate::vocabulary::ValueList(<$name>::tokens())
                        )
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

pub(crate) use closed_vocabulary;
