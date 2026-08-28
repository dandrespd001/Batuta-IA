//! Los vocabularios cerrados de batuta.
//!
//! Cada uno está medido, no inventado: sale de `models.toml`, de `roles.toml` o
//! del esquema de manifiesto del brief §4. **Un valor que no está aquí no
//! existe**, y R2 manda: no se añade una variante «por si acaso», se añade
//! cuando hay un manifiesto y un canario que la demuestren.

use crate::vocabulary::closed_vocabulary;

closed_vocabulary! {
    /// Rol del trabajo: **lo que la tarea necesita**, no cómo se llama.
    ///
    /// Los 18 roles de `roles.toml` [VERIFICADO 2026-08-25]. La trampa medida:
    /// elegir `tests` porque el encargo trae tests manda el trabajo a la ruta de
    /// los gratuitos aunque haya que compilar y ejecutar.
    Role = "role" {
        /// Diseño de sistemas y decisiones normativas.
        Architecture = "architecture",
        /// Análisis estadístico y cuantitativo.
        Statistics = "statistics",
        /// Revisión y trabajo de seguridad.
        Security = "security",
        /// Implementación general de funcionalidad.
        Implementation = "implementation",
        /// Escritura de pruebas.
        Tests = "tests",
        /// Código repetitivo de forma conocida.
        Boilerplate = "boilerplate",
        /// Trabajo que necesita ventana de contexto grande.
        LongContext = "long_context",
        /// Investigación con fuentes. Exige capacidad `web_research` demostrada.
        Research = "research",
        /// Documentación.
        Docs = "docs",
        /// Refactor mecánico a gran escala.
        BulkRefactor = "bulk_refactor",
        /// Segunda lectura de trabajo ajeno.
        Review = "review",
        /// C++ acotado por firmas y pruebas.
        BoundedCpp = "bounded_cpp",
        /// Determinismo, save/replay, RNG, checksum, scheduler, command stream.
        Determinism = "determinism",
        /// Interfaz y escena de Godot.
        GodotUi = "godot_ui",
        /// Diseño visual o de interacción.
        Design = "design",
        /// Edición de contenido redactado.
        ContentEdit = "content_edit",
        /// Análisis de datos.
        DataAnalysis = "data_analysis",
        /// Trabajo con más de una modalidad de entrada.
        Multimodal = "multimodal",
    }
}

closed_vocabulary! {
    /// Nivel de sensibilidad del material que la tarea toca.
    ///
    /// **El orden de declaración es la política.** `Ord` sale de aquí y
    /// `<=` significa «cabe dentro del techo». Reordenar estas variantes cambia
    /// quién puede ver qué; hay una prueba que lo fija.
    Sensitivity = "sensitivity" {
        /// Material publicable.
        Public = "public",
        /// Material del proyecto, no publicable.
        Internal = "internal",
        /// Material restringido dentro del proyecto.
        Confidential = "confidential",
        /// Credenciales y material de autenticación.
        Secrets = "secrets",
        /// Control financiero.
        FinancialControl = "financial_control",
        /// Despliegue y producción.
        Deployment = "deployment",
    }
}

impl Sensitivity {
    /// Posición en la escala, empezando en cero por `public`.
    ///
    /// Coincide con el índice en [`Sensitivity::ALL`]: el orden de declaración
    /// es el de la política y hay una prueba que lo fija.
    pub const fn rank(self) -> u8 {
        match self {
            Self::Public => 0,
            Self::Internal => 1,
            Self::Confidential => 2,
            Self::Secrets => 3,
            Self::FinancialControl => 4,
            Self::Deployment => 5,
        }
    }

    /// ¿Cabe este nivel bajo `ceiling`?
    ///
    /// Un techo admite lo suyo y todo lo de debajo. Es la comparación que decide
    /// si un modelo con `max_sensitivity = "internal"` puede ver una tarea, y no
    /// hay ninguna otra: sin clasificadores, sin subcadenas (R5).
    pub const fn fits_within(self, ceiling: Self) -> bool {
        self.rank() <= ceiling.rank()
    }
}

closed_vocabulary! {
    /// Forma que debe tener la salida aceptada.
    ///
    /// Los cuatro valores del error de R8. `patch` **no** es uno de ellos.
    OutputContract = "output_contract" {
        /// Texto plano.
        Text = "text",
        /// Un documento JSON.
        Json = "json",
        /// Un diff unificado aplicable.
        UnifiedDiff = "unified_diff",
        /// Un dictamen de revisión.
        Review = "review",
    }
}

closed_vocabulary! {
    /// Qué se le permite hacer a la tarea con el árbol de trabajo.
    WriteMode = "write_mode" {
        /// No escribe. Investigación y dictámenes.
        ReadOnly = "read_only",
        /// Produce un parche que el Arquitecto revisa. Nunca autoaplica.
        ValidatedPatch = "validated_patch",
        /// Aplica tras pasar los gates, en árbol aislado.
        ValidatedApply = "validated_apply",
    }
}

closed_vocabulary! {
    /// Cómo escribe un modelo, si es que escribe.
    ///
    /// La distinción cara: `native_writer` autoaplica al repositorio de origen;
    /// `native_patch_writer` edita el árbol desechable igual de libremente
    /// —puede iterar y hacer fase roja— pero termina siempre en `PATCH_READY`.
    /// Los dos privilegios venían empaquetados en uno solo y separarlos costó
    /// un incidente.
    ExecutionProfile = "execution_profile" {
        /// No edita nada.
        ReadOnly = "read_only",
        /// Edita el árbol y puede autoaplicar al repositorio de origen.
        NativeWriter = "native_writer",
        /// Edita el árbol desechable; entrega siempre parche revisable.
        NativePatchWriter = "native_patch_writer",
        /// Escribe por API de parches, sin árbol propio.
        PatchApiWriter = "patch_api_writer",
        /// Escribe por el protocolo ACP de Kimi.
        KimiAcpWriter = "kimi_acp_writer",
    }
}

closed_vocabulary! {
    /// Capacidad que una tarea exige y que un modelo debe **demostrar** (R2).
    ///
    /// El fallo que lo paga: `web_research` figuraba declarada en un solo
    /// modelo, su transporte no navega, y la delegación hizo cero llamadas a
    /// herramientas mientras producía 38 KB con veinte citas.
    Capability = "capability" {
        /// Leer los ficheros de entrada.
        Read = "read",
        /// Escribir en el árbol de trabajo.
        Write = "write",
        /// Invocar herramientas del propio CLI.
        Tools = "tools",
        /// Navegar y citar fuentes externas.
        WebResearch = "web_research",
    }
}

closed_vocabulary! {
    /// Esfuerzo de razonamiento pedido al modelo.
    ///
    /// Orden de declaración = orden creciente de esfuerzo.
    ReasoningEffort = "reasoning_effort" {
        /// Mínimo. Canarios y comprobaciones baratas.
        Low = "low",
        /// Intermedio.
        Medium = "medium",
        /// Trabajo normal.
        High = "high",
        /// Arquitectura, determinismo, cambios normativos.
        Xhigh = "xhigh",
        /// Todo lo que el proveedor permita.
        Max = "max",
    }
}

closed_vocabulary! {
    /// Origen del acceso al proveedor, a efectos de confianza.
    TrustTier = "trust_tier" {
        /// Suscripción oficial de pago.
        OfficialSubscription = "official_subscription",
        /// Ejecución local.
        Local = "local",
        /// Acceso gratuito por web.
        WebFree = "web_free",
    }
}

closed_vocabulary! {
    /// Cómo se autentica un proveedor.
    ///
    /// R10 — «un secreto, un nombre, una vez»: cuando es `sealed_credential`, el
    /// nombre sellado sale del manifiesto y de ningún otro sitio. Buscar
    /// `deepseek-api-key` lo que se selló como `qwen-deepseek-api-key` costó
    /// semanas sin credencial con una clave válida en la máquina.
    AuthMethod = "auth_method" {
        /// El CLI del proveedor guarda su propia sesión OAuth.
        OauthCli = "oauth_cli",
        /// Credencial sellada con `systemd-creds`, nombrada en el manifiesto.
        SealedCredential = "sealed_credential",
    }
}

closed_vocabulary! {
    /// Naturaleza del proveedor.
    ///
    /// Sólo `cli` porque sólo `cli` está demostrado (R2). Añadir `http` exige
    /// manifiesto y canario que lo ejerzan, no una variante de más aquí.
    ProviderKind = "provider_kind" {
        /// Un ejecutable local que se invoca por argv.
        Cli = "cli",
    }
}

closed_vocabulary! {
    /// Cómo se extrae el artefacto del flujo crudo del transporte (R14).
    ///
    /// El fallo que lo paga: el rescate de una delegación escribió 111 KB de
    /// NDJSON de `OpenCode` en tres líneas. **El artefacto se entrega extraído.**
    ///
    /// Son cuatro genéricos, no uno por proveedor: los siete parsers ad hoc del
    /// orquestador viejo (`codex_jsonl`, `claude_json`, `harness_json`,
    /// `qwen_json`, `kimi_text`, `abacus_text`, `fx_json`) eran el mismo puñado
    /// de formas con distinto nombre. Lo que no encaje va a `plugin`, fuera de
    /// proceso, sin tocar el núcleo.
    ParserKind = "parser_kind" {
        /// La salida entera es el artefacto.
        PlainText = "plain_text",
        /// NDJSON: el texto del último registro que lo lleve.
        JsonlLastText = "jsonl_last_text",
        /// Un JSON del que se extrae un puntero concreto.
        JsonPointer = "json_pointer",
        /// Un plugin externo, por la ABI C, decide.
        Plugin = "plugin",
    }
}

closed_vocabulary! {
    /// Cómo llega el prompt al ejecutable.
    ///
    /// Nunca por argv cuando hay material sensible: argv es visible en `ps` para
    /// cualquier proceso del mismo usuario. Esa regla no se queda en el
    /// comentario: la fija [`PromptDelivery::max_sensitivity`].
    PromptDelivery = "prompt_delivery" {
        /// Por entrada estándar.
        Stdin = "stdin",
        /// Por fichero temporal, con la bandera que diga el manifiesto.
        File = "file",
        /// Por los argumentos posicionales del propio ejecutable.
        ///
        /// Medido el 2026-08-27: `dsh --profile headless` rechaza el prompt por
        /// entrada estándar (`error: a task is required`, exit 1) y no tiene
        /// bandera de fichero. Es la única vía del único transporte demostrado,
        /// así que la variante no es una previsión: es lo que hay.
        Argv = "argv",
    }
}

impl PromptDelivery {
    /// Sensibilidad máxima que esta vía puede llevar.
    ///
    /// `argv` se queda en `internal` porque la línea de órdenes de un proceso la
    /// lee cualquier otro proceso del mismo usuario; `stdin` y un fichero en
    /// modo 0600 no se ven desde fuera y llegan hasta arriba de la escala.
    ///
    /// El techo vive aquí, y no en el manifiesto ni en la política, por la misma
    /// razón que las capacidades implícitas del `TaskSpec` se derivan en un solo
    /// sitio: cada lugar donde hubiera que repetir la regla es un lugar donde
    /// podría divergir. Y una divergencia aquí no es un error de forma, es un
    /// prompt confidencial en la tabla de procesos.
    pub const fn max_sensitivity(self) -> Sensitivity {
        match self {
            Self::Argv => Sensitivity::Internal,
            Self::Stdin | Self::File => Sensitivity::Deployment,
        }
    }

    /// ¿Puede esta vía llevar material de esta sensibilidad?
    pub const fn admits(self, sensitivity: Sensitivity) -> bool {
        sensitivity.fits_within(self.max_sensitivity())
    }
}

closed_vocabulary! {
    /// Formato en que se materializa un fichero de configuración por corrida.
    ///
    /// Sólo `yaml` porque sólo `yaml` está demostrado: los dos manifiestos que
    /// existen escriben YAML, y dsh es quien lo lee. Añadir `json` o `toml`
    /// «porque son fáciles» sería exactamente la variante por si acaso que R2
    /// prohíbe —la misma razón por la que [`ProviderKind`] tiene un solo valor.
    DocumentFormat = "document_format" {
        /// YAML.
        Yaml = "yaml",
    }
}

closed_vocabulary! {
    /// De dónde sale la procedencia que se anota en el recibo.
    ///
    /// El fallo que lo paga, medido el 2026-08-27: batuta pidió
    /// `deepseek-v4-flash` tres veces y las tres corrió `MiniMax-M2.7`, porque
    /// el modelo lo decidía un fichero que batuta no controlaba. Un recibo que
    /// anota lo pedido en vez de lo ocurrido **miente**, y miente justo sobre lo
    /// que da valor al recibo.
    ///
    /// Los dos valores no son equivalentes y por eso son dos: `session_log` es
    /// observacional y `declared` es una promesa que sólo el canario contrasta.
    /// Un proveedor que no ofrezca registro legible es más débil, y el
    /// manifiesto tiene que decirlo en vez de disimularlo.
    ProvenanceSource = "provenance_source" {
        /// El proveedor deja un registro legible con el modelo que corrió.
        SessionLog = "session_log",
        /// El proveedor nombra el modelo en su **stderr**, con un patrón que el
        /// manifiesto declara.
        ///
        /// No es una hipótesis: `abacusai` escribe `model: ZAI_GLM_5_3_FLASH |
        /// cwd: … | conversation: …`. Nueve canarios salieron «sin confirmar»
        /// teniendo la respuesta delante. Es tan observacional como
        /// `session_log`: lo escribe la máquina, no batuta.
        StderrPattern = "stderr_pattern",
        /// No hay registro: se anota lo pedido y lo contrasta el canario.
        Declared = "declared",
    }
}

impl ProvenanceSource {
    /// ¿Se puede **comprobar** qué modelo corrió, o sólo creerlo?
    ///
    /// Es la pregunta que separa un verde de «el transporte funciona» de un
    /// verde de «corrió lo que se pidió». Las dos son resultados y hay que poder
    /// distinguirlas sin deducirlas.
    pub const fn es_observable(self) -> bool {
        matches!(self, Self::SessionLog | Self::StderrPattern)
    }
}

closed_vocabulary! {
    /// Qué comprueba un canario, de forma **observacional** (R2, R3, R4).
    ///
    /// No se juzga por subcadena. R3 se paga aquí: `provider-canary` devolvió
    /// `QUOTA_UNAVAILABLE` en 126 ms sin tocar la red, porque leyó su propio
    /// reflejo en la política que él mismo debía informar.
    CanaryExpectation = "canary_expectation" {
        /// El modelo devuelve exactamente el token que se le dio.
        TokenEcho = "token_echo",
    }
}
