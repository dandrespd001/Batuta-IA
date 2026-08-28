//! `batuta panel`: la tabla que une declaración, evidencia y elección.
//!
//! §1 de `docs/FASE5_PANEL.md`: **declaración** (`providers/*.toml`),
//! **evidencia** (los recibos de `batuta canary`) y **elección** (la
//! política). Un modelo declarado y con evidencia puede seguir sin ser el que
//! queremos; uno que queremos puede no tener evidencia todavía. El panel no
//! decide nada: enseña las tres capas juntas para que quien lo lea no tenga
//! que ir a buscarlas por separado.

use std::fmt::Write as _;
use std::path::Path;
use std::time::{Duration, SystemTime};

use batuta_manifest::{ModelEntry, ProviderManifest};
use batuta_policy::Politica;
use batuta_store::{LatestGreen, ReceiptStore};

use crate::error::CliError;
use crate::paths::Layout;

/// Una fila: un modelo, con lo que dicen sus tres capas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fila {
    /// El proveedor que lo declara.
    pub provider: String,
    /// El modelo, con el identificador de batuta.
    pub model: String,
    /// El esfuerzo elegido, si la política fija uno.
    pub effort: Option<String>,
    /// Si la política lo tiene activo.
    pub enabled: bool,
    /// Si de verdad se puede enrutar hoy: activo **y** con evidencia fresca.
    ///
    /// No se deduce de las otras columnas al leer la tabla: se calcula aquí y
    /// se enseña aparte, porque «activo» y «enrutable» no son lo mismo —un
    /// modelo activo sin ningún canario verde no es enrutable, por mucho que
    /// la elección diga que sí (§1)— y esa distinción es la que un vistazo a
    /// `ESTADO` y `CANARIO` por separado puede pasar por alto.
    pub routable: bool,
    /// La columna de canario, ya redactada: `ninguno`, `verde hace 2 h`,
    /// `caducado hace 3 d`.
    pub canary: String,
    /// Si el modelo que corrió quedó **confirmado**. `None` sin ningún recibo
    /// que lo diga.
    pub confirmed: Option<bool>,
    /// La marca `⚠`, con el nombre declarado, cuando `observed_as` difiere de
    /// `route_model`: ese modelo no se confirma por eco literal, sino por el
    /// alias que el manifiesto declaró.
    pub warning: Option<String>,
}

/// Construye una fila por modelo, de los manifiestos que pasen el filtro.
///
/// Los manifiestos se releen en cada invocación (R7), igual que `canary`.
///
/// # Errors
///
/// [`CliError::Manifest`] si un manifiesto no carga, [`CliError::UnknownProvider`]
/// si `provider_filter` no nombra ninguno, [`CliError::Policy`] si la política
/// existe y está rota (no si no existe: eso es el estado inicial, sin nada
/// activo), [`CliError::Store`] si el almacén de recibos no se pudo listar.
pub fn filas(
    providers_dir: &Path,
    layout: &Layout,
    provider_filter: Option<&str>,
) -> Result<Vec<Fila>, CliError> {
    let manifiestos = crate::command::cargar(providers_dir)?;

    if let Some(filtro) = provider_filter
        && !manifiestos.iter().any(|m| m.id().as_str() == filtro)
    {
        let mut available: Vec<String> = manifiestos
            .iter()
            .map(|m| m.id().as_str().to_string())
            .collect();
        available.sort();
        return Err(CliError::UnknownProvider {
            asked: filtro.to_string(),
            available,
        });
    }

    let politica = cargar_politica(layout)?;
    let recibos = ReceiptStore::open(layout.receipts());

    let mut filas = Vec::new();
    for manifiesto in &manifiestos {
        if provider_filter.is_some_and(|filtro| manifiesto.id().as_str() != filtro) {
            continue;
        }
        for modelo in manifiesto.models() {
            filas.push(fila_de(manifiesto, modelo, &politica, &recibos)?);
        }
    }
    Ok(filas)
}

/// La política, o la de un primer arranque si el fichero no existe todavía.
///
/// Un fichero ausente no es un error: es el estado antes de que nadie haya
/// tocado `enable`, `disable` o `effort`. Un fichero **roto**, en cambio, sí
/// lo es —no se sustituye en silencio por una política vacía. Compartida con
/// `eleccion` (`enable`/`disable`/`effort` la leen igual antes de mutarla).
pub(crate) fn cargar_politica(layout: &Layout) -> Result<Politica, CliError> {
    match Politica::cargar(&layout.politica()) {
        Ok(politica) => Ok(politica),
        Err(batuta_policy::PoliticaError::Read { ref source, .. })
            if source.kind() == std::io::ErrorKind::NotFound =>
        {
            Ok(Politica::vacia())
        }
        Err(source) => Err(CliError::Policy {
            source: Box::new(source),
        }),
    }
}

fn fila_de(
    manifiesto: &ProviderManifest,
    modelo: &ModelEntry,
    politica: &Politica,
    recibos: &ReceiptStore,
) -> Result<Fila, CliError> {
    let enabled = politica.esta_habilitado(modelo.id());
    let effort = politica.esfuerzo(modelo.id()).map(|e| e.to_string());

    let consulta = recibos
        .latest_green(
            modelo.id().as_str(),
            manifiesto.source_sha256(),
            batuta_store::DEFAULT_TTL,
        )
        .map_err(|e| CliError::Store {
            source: Box::new(e),
        })?;

    let (canary, confirmed, routable) = match consulta.result {
        LatestGreen::Fresh { receipt, sealed_at } => (
            format!("verde {}", hace(sealed_at)),
            Some(receipt.model_confirmed()),
            enabled,
        ),
        LatestGreen::Expired { at } => (format!("caducado {}", hace(at)), None, false),
        LatestGreen::Absent => ("ninguno".to_string(), None, false),
    };

    let warning = if modelo
        .observed_as()
        .is_some_and(|alias| alias != modelo.route_model().as_str())
    {
        modelo.observed_as().map(std::string::ToString::to_string)
    } else {
        None
    };

    Ok(Fila {
        provider: manifiesto.id().as_str().to_string(),
        model: modelo.id().as_str().to_string(),
        effort,
        enabled,
        routable,
        canary,
        confirmed,
        warning,
    })
}

/// «Hace cuánto», en prosa breve. Sin dependencia nueva (T4 lo exige): un
/// cálculo de cuatro tramos basta para lo que el panel necesita mostrar.
fn hace(instante: SystemTime) -> String {
    let transcurrido = SystemTime::now()
        .duration_since(instante)
        .unwrap_or(Duration::ZERO);
    let segundos = transcurrido.as_secs();

    if segundos < 60 {
        "hace unos segundos".to_string()
    } else if segundos < 60 * 60 {
        format!("hace {} min", segundos / 60)
    } else if segundos < 60 * 60 * 24 {
        format!("hace {} h", segundos / (60 * 60))
    } else {
        format!("hace {} d", segundos / (60 * 60 * 24))
    }
}

/// El ancho de cada columna, calculado a mano sobre las filas reales: sin
/// dependencia nueva, ninguna tabla ni formateador. Cabecera incluida, para
/// que la cabecera nunca quede más corta que su columna.
struct Anchos {
    provider: usize,
    model: usize,
    effort: usize,
    enabled: usize,
    routable: usize,
    canary: usize,
}

impl Anchos {
    fn de(filas: &[Fila]) -> Self {
        let mut anchos = Self {
            provider: "PROVEEDOR".len(),
            model: "MODELO".len(),
            effort: "ESFUERZO".len(),
            enabled: "ACTIVO".len(),
            routable: "ENRUTABLE".len(),
            canary: "CANARIO".len(),
        };
        for fila in filas {
            anchos.provider = anchos.provider.max(fila.provider.len());
            anchos.model = anchos.model.max(fila.model.len());
            anchos.effort = anchos
                .effort
                .max(fila.effort.as_deref().unwrap_or("—").len());
            anchos.canary = anchos.canary.max(fila.canary.len());
        }
        anchos
    }
}

/// Redacta la tabla entera, una línea por fila.
///
/// La columna `CONFIRMADO` y la marca `⚠` van al final de la línea, sin ancho
/// fijo: son la parte que sólo aparece cuando hay algo que decir, y forzarlas
/// a una columna alinearía espacio que nadie usaría en la mayoría de filas.
pub fn tabla(filas: &[Fila]) -> String {
    let anchos = Anchos::de(filas);
    let mut salida = String::new();

    let _ = writeln!(
        salida,
        "{:<prov$}  {:<modl$}  {:<esf$}  {:<act$}  {:<enr$}  {:<can$}  CONFIRMADO",
        "PROVEEDOR",
        "MODELO",
        "ESFUERZO",
        "ACTIVO",
        "ENRUTABLE",
        "CANARIO",
        prov = anchos.provider,
        modl = anchos.model,
        esf = anchos.effort,
        act = anchos.enabled,
        enr = anchos.routable,
        can = anchos.canary,
    );

    for fila in filas {
        let esfuerzo = fila.effort.as_deref().unwrap_or("—");
        let activo = if fila.enabled { "sí" } else { "no" };
        let enrutable = if fila.routable { "sí" } else { "no" };
        let confirmado = match fila.confirmed {
            Some(true) => "confirmado",
            Some(false) => "sin confirmar",
            None => "—",
        };

        let _ = write!(
            salida,
            "{:<prov$}  {:<modl$}  {:<esf$}  {:<act$}  {:<enr$}  {:<can$}  {confirmado}",
            fila.provider,
            fila.model,
            esfuerzo,
            activo,
            enrutable,
            fila.canary,
            prov = anchos.provider,
            modl = anchos.model,
            esf = anchos.effort,
            act = anchos.enabled,
            enr = anchos.routable,
            can = anchos.canary,
        );
        if let Some(alias) = &fila.warning {
            let _ = write!(salida, "  ⚠ {alias}");
        }
        salida.push('\n');
    }

    salida
}

/// Escapa `&`, `<`, `>` y `"` para insertar texto libre del manifiesto
/// (`provider`, `model`, el alias de `warning`) dentro de HTML sin romper la
/// estructura de la página. El orden importa: `&` va primero, porque un `<`
/// ya escapado a `&lt;` volvería a escaparse a `&amp;lt;` si `&` se
/// procesara después.
fn escapar_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// La frase de sólo lectura, visible en el cuerpo de la página (T7): no es un
/// adorno de un comentario HTML, es el hecho central que la página tiene que
/// comunicar —es una fotografía de un instante, no un panel de control.
const AVISO_SOLO_LECTURA: &str = "Esta página es de sólo lectura: es una fotografía de este instante \
     y no aplica ningún cambio. Para activar, desactivar o cambiar el \
     esfuerzo de un modelo, usa <code>batuta enable</code>, \
     <code>batuta disable</code> o <code>batuta effort</code> desde la \
     línea de órdenes.";

/// La misma tabla que [`tabla`], como página HTML autocontenida: sin red,
/// sin CDN, sin fuente externa (T7, `docs/FASE5_PANEL.md`).
///
/// Recibe las mismas `filas` que [`tabla`] — quien llama debe construirlas
/// con una única invocación de [`filas()`] y pasar esa misma variable a las
/// dos funciones, nunca dos llamadas separadas. `Fila.canary` viene de
/// [`hace`], prosa relativa a «ahora» (`hace 2 min`, `hace 3 h`): dos
/// llamadas a `filas()` en instantes distintos podrían divergir en el borde
/// de un minuto, y entonces la tabla de texto y la de HTML contarían
/// versiones distintas de la misma corrida. Es la garantía de «la misma
/// verdad» que el checklist de T7 pide.
pub fn tabla_html(filas: &[Fila]) -> String {
    let aviso = AVISO_SOLO_LECTURA;
    let mut cuerpo = String::new();
    for fila in filas {
        let esfuerzo = fila.effort.as_deref().unwrap_or("—");
        let activo = if fila.enabled { "sí" } else { "no" };
        let enrutable = if fila.routable { "sí" } else { "no" };
        let confirmado = match fila.confirmed {
            Some(true) => "confirmado",
            Some(false) => "sin confirmar",
            None => "—",
        };
        let alias = fila
            .warning
            .as_deref()
            .map_or_else(|| "—".to_string(), |alias| format!("⚠ {alias}"));

        let _ = writeln!(
            cuerpo,
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            escapar_html(&fila.provider),
            escapar_html(&fila.model),
            escapar_html(esfuerzo),
            activo,
            enrutable,
            escapar_html(&fila.canary),
            confirmado,
            escapar_html(&alias),
        );
    }

    format!(
        "<!doctype html>\n\
         <html lang=\"es\">\n\
         <head>\n\
         <meta charset=\"utf-8\">\n\
         <title>batuta panel</title>\n\
         <style>\n\
         body {{ font-family: monospace; margin: 2rem; }}\n\
         table {{ border-collapse: collapse; margin-top: 1rem; }}\n\
         th, td {{ border: 1px solid #999; padding: 0.3rem 0.6rem; text-align: left; }}\n\
         th {{ background: #eee; }}\n\
         p.aviso {{ font-weight: bold; }}\n\
         </style>\n\
         </head>\n\
         <body>\n\
         <h1>batuta panel</h1>\n\
         <p class=\"aviso\">{aviso}</p>\n\
         <table>\n\
         <thead>\n\
         <tr><th>PROVEEDOR</th><th>MODELO</th><th>ESFUERZO</th><th>ACTIVO</th>\
         <th>ENRUTABLE</th><th>CANARIO</th><th>CONFIRMADO</th><th>ALIAS</th></tr>\n\
         </thead>\n\
         <tbody>\n\
         {cuerpo}\
         </tbody>\n\
         </table>\n\
         </body>\n\
         </html>\n"
    )
}

/// Escribe [`tabla_html`] de `filas` en `ruta` (§2/§3 de
/// `docs/FASE5_PANEL.md`: `batuta panel --html <ruta>`).
///
/// Vive junto a `tabla_html`, no en `main.rs`, con el mismo reparto que ya
/// usa [`crate::declaracion::nuevo_proveedor`]: quien construye el contenido
/// también hace el `fs::write` y traduce el error de disco a
/// [`CliError::Io`], para que `main.rs` sólo tenga que decidir qué imprimir
/// según el resultado.
///
/// # Errors
///
/// [`CliError::Io`] si no se pudo escribir en `ruta` (directorio inexistente,
/// permisos, disco lleno...).
pub fn escribir_html(ruta: &Path, filas: &[Fila]) -> Result<(), CliError> {
    std::fs::write(ruta, tabla_html(filas)).map_err(|source| CliError::Io {
        path: ruta.to_path_buf(),
        source,
    })
}
