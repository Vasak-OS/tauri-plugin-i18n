// TAKEN FROM rust-i18n-support library

use normpath::PathExt;
use std::fs::File;
use std::io::prelude::*;
use std::{collections::BTreeMap, path::Path};

type Locale = String;
type Value = serde_json::Value;
type Translations = BTreeMap<Locale, Value>;

include!(concat!(env!("OUT_DIR"), "/bundled_locales.rs"));

pub fn load_data(runtime_path: Option<&str>) -> BTreeMap<String, BTreeMap<String, String>> {
    let mut final_result: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();

    //Load the Bundled Data (Compile-time)
    // get_bundled_data() returns Vec<(locale_name, extension, file_content)>
    for (locale, ext, content) in get_bundled_data() {
        if let Ok(trs) = parse_file(content, ext, locale) {
            trs.into_iter().for_each(|(loc, val)| {
                let flattened = flatten_keys("", &val);
                final_result.entry(loc).or_default().extend(flattened);
            });
        }
    }

    //Add runtime overrides if any
    if let Some(path) = runtime_path {
        let external = load_locales(path, |_| false);
        for (locale, keys) in external {
            final_result.entry(locale).or_default().extend(keys);
        }
    }

    final_result
}

pub fn is_debug() -> bool {
    std::env::var("RUST_I18N_DEBUG").unwrap_or_else(|_| "0".to_string()) == "1"
}

// Load locales into flatten key, value HashMap
fn load_locales<F: Fn(&str) -> bool>(
    locales_path: &str,
    ignore_if: F,
) -> BTreeMap<String, BTreeMap<String, String>> {
    let mut result: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    let mut translations = BTreeMap::new();
    let locales_path = match Path::new(locales_path).normalize() {
        Ok(p) => p,
        Err(e) => {
            if is_debug() {
                println!("cargo:i18n-error={}", e);
            }
            return result;
        }
    };
    let locales_path = match locales_path.as_path().to_str() {
        Some(p) => p,
        None => {
            if is_debug() {
                println!("cargo:i18n-error=could not convert path");
            }
            return result;
        }
    };

    let path_pattern = format!("{locales_path}/**/*.{{yml,yaml,json,toml}}");

    if is_debug() {
        println!("cargo:i18n-locale={}", path_pattern);
    }

    // check dir exists
    if !Path::new(locales_path).exists() {
        if is_debug() {
            println!("cargo:i18n-error=path not exists: {}", locales_path);
        }
        return result;
    }

    // Nada de lo que pase con un archivo suelto puede tumbar la aplicación.
    //
    // Esto entraba en pánico por seis motivos distintos: el glob, el nombre sin
    // punto, la extensión ausente, el archivo ilegible, el contenido que no es
    // UTF-8 y el catálogo mal formado. Cualquiera de ellos y **la aplicación no
    // abre** — pantalla negra, sin ventana y sin explicación. Un catálogo roto
    // tiene que degradar los textos, no impedir usar el programa: para eso están
    // los empaquetados dentro del binario.
    let entradas = match globwalk::glob(&path_pattern) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("[i18n] patrón de búsqueda inválido ({path_pattern}): {e}");
            return result;
        }
    };

    for entry in entradas {
        let entry = match entry {
            Ok(e) => e.into_path(),
            Err(e) => {
                eprintln!("[i18n] no se pudo recorrer {locales_path}: {e}");
                continue;
            }
        };
        if is_debug() {
            println!("cargo:i18n-load={}", entry.display());
        }

        if ignore_if(&entry.display().to_string()) {
            continue;
        }

        let Some(locale) = entry
            .file_stem()
            .and_then(|s| s.to_str())
            .and_then(|s| s.split('.').next_back())
            .filter(|l| !l.is_empty())
        else {
            eprintln!("[i18n] se salta {}: no se pudo deducir el idioma", entry.display());
            continue;
        };

        let Some(ext) = entry.extension().and_then(|s| s.to_str()) else {
            continue;
        };

        let mut content = String::new();
        match File::open(&entry).and_then(|f| std::io::BufReader::new(f).read_to_string(&mut content))
        {
            Ok(_) => {}
            Err(e) => {
                eprintln!("[i18n] se salta {}: {e}", entry.display());
                continue;
            }
        }

        let trs = match parse_file(&content, ext, locale) {
            Ok(trs) => trs,
            Err(e) => {
                eprintln!("[i18n] se salta {}: {e}", entry.display());
                continue;
            }
        };

        trs.into_iter().for_each(|(k, new_value)| {
            translations
                .entry(k)
                .and_modify(|old_value| merge_value(old_value, &new_value))
                .or_insert(new_value);
        });
    }

    translations.iter().for_each(|(locale, trs)| {
        result.insert(locale.to_string(), flatten_keys("", trs));
    });

    result
}

/// Merge JSON Values, merge b into a
fn merge_value(a: &mut Value, b: &Value) {
    match (a, b) {
        (Value::Object(a), Value::Object(b)) => {
            for (k, v) in b {
                merge_value(a.entry(k.clone()).or_insert(Value::Null), v);
            }
        }
        (a, b) => {
            *a = b.clone();
        }
    }
}

// Parse Translations from file to support multiple formats
fn parse_file(content: &str, ext: &str, locale: &str) -> Result<Translations, String> {
    let result = match ext {
        "yml" | "yaml" => serde_yaml::from_str::<serde_json::Value>(content)
            .map_err(|err| format!("Invalid YAML format, {}", err)),
        "json" => serde_json::from_str::<serde_json::Value>(content)
            .map_err(|err| format!("Invalid JSON format, {}", err)),
        "toml" => toml::from_str::<serde_json::Value>(content)
            .map_err(|err| format!("Invalid TOML format, {}", err)),
        _ => Err("Invalid file extension".into()),
    };

    match result {
        Ok(v) => match get_version(&v) {
            2 => {
                if let Some(trs) = parse_file_v2("", &v) {
                    return Ok(trs);
                }

                Err("Invalid locale file format, please check the version field".into())
            }
            _ => Ok(parse_file_v1(locale, &v)),
        },
        Err(e) => Err(e),
    }
}

/// Locale file format v1
///
/// For example:
/// ```yml
/// welcome: Welcome
/// foo: Foo bar
/// ```
fn parse_file_v1(locale: &str, data: &serde_json::Value) -> Translations {
    Translations::from([(locale.to_string(), data.clone())])
}

/// Locale file format v2
/// Iter all nested keys, if the value is not a object (Map<locale, string>), then convert into multiple locale translations
///
/// If the final value is Map<locale, string>, then convert them and insert into trs
///
/// For example (only support 1 level):
///
/// ```yml
/// _version: 2
/// welcome.first:
///   en: Welcome
///   zh-CN: 欢迎
/// welcome1:
///   en: Welcome 1
///   zh-CN: 欢迎 1
/// ```
///
/// into
///
/// ```yml
/// en.welcome.first: Welcome
/// zh-CN.welcome.first: 欢迎
/// en.welcome1: Welcome 1
/// zh-CN.welcome1: 欢迎 1
/// ```
fn parse_file_v2(key_prefix: &str, data: &serde_json::Value) -> Option<Translations> {
    let mut trs = Translations::new();

    if let serde_json::Value::Object(messages) = data {
        for (key, value) in messages {
            if let serde_json::Value::Object(sub_messages) = value {
                // If all values are string, then convert them into multiple locale translations
                for (locale, text) in sub_messages {
                    // Ignore if the locale is not a locale
                    // e.g:
                    //  en: Welcome
                    //  zh-CN: 欢迎
                    if text.is_string() {
                        let key = format_keys(&[key_prefix, key]);
                        let sub_trs = BTreeMap::from([(key, text.clone())]);
                        let sub_value = serde_json::to_value(&sub_trs).unwrap();

                        trs.entry(locale.clone())
                            .and_modify(|old_value| merge_value(old_value, &sub_value))
                            .or_insert(sub_value);
                        continue;
                    }

                    if text.is_object() {
                        // Parse the nested keys
                        // If the value is object (Map<locale, string>), iter them and convert them and insert into trs
                        let key = format_keys(&[key_prefix, key]);
                        if let Some(sub_trs) = parse_file_v2(&key, value) {
                            // Merge the sub_trs into trs
                            for (locale, sub_value) in sub_trs {
                                trs.entry(locale)
                                    .and_modify(|old_value| merge_value(old_value, &sub_value))
                                    .or_insert(sub_value);
                            }
                        }
                    }
                }
            }
        }
    }

    if !trs.is_empty() {
        return Some(trs);
    }

    None
}

/// Get `_version` from JSON root
/// If `_version` is not found, then return 1 as default.
fn get_version(data: &serde_json::Value) -> usize {
    if let Some(version) = data.get("_version") {
        return version.as_u64().unwrap_or(1) as usize;
    }

    1
}

/// Join the keys with dot, if any key is empty, omit it.
fn format_keys(keys: &[&str]) -> String {
    keys.iter()
        .filter(|k| !k.is_empty())
        .map(|k| k.to_string())
        .collect::<Vec<String>>()
        .join(".")
}

fn flatten_keys(prefix: &str, trs: &Value) -> BTreeMap<String, String> {
    let mut v = BTreeMap::<String, String>::new();
    let prefix = prefix.to_string();

    match &trs {
        serde_json::Value::String(s) => {
            v.insert(prefix, s.to_string());
        }
        serde_json::Value::Object(o) => {
            for (k, vv) in o {
                let key = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{}.{}", prefix, k)
                };
                v.extend(flatten_keys(key.as_str(), vv));
            }
        }
        serde_json::Value::Null => {
            v.insert(prefix, "".into());
        }
        serde_json::Value::Bool(s) => {
            v.insert(prefix, format!("{}", s));
        }
        serde_json::Value::Number(s) => {
            v.insert(prefix, format!("{}", s));
        }
        // Un arreglo no es un texto traducible. Antes se guardaba como cadena
        // vacía, así que la clave *existía* con contenido vacío: la interfaz
        // mostraba un hueco y no había forma de notar que el catálogo estaba mal.
        // Sin guardarla, la clave falta y se ve cruda, que es lo que hace que
        // alguien la arregle.
        serde_json::Value::Array(_) => {}
    }

    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn json(texto: &str) -> Value {
        serde_json::from_str(texto).expect("json de prueba")
    }

    /// Un directorio de catálogos de mentira, propio de cada prueba.
    ///
    /// Con nombre por prueba porque corren en paralelo en el mismo proceso: un
    /// directorio compartido hace que se borren los archivos entre ellas y el fallo
    /// aparece y desaparece según el orden.
    fn con_catalogos(quien: &str, archivos: &[(&str, &str)]) -> std::path::PathBuf {
        let base =
            std::env::temp_dir().join(format!("i18n-cat-{}-{quien}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        for (nombre, contenido) in archivos {
            std::fs::write(base.join(nombre), contenido).unwrap();
        }
        base
    }

    #[test]
    fn las_claves_anidadas_se_aplanan_con_puntos() {
        // Es el corazón del plugin: el catálogo se escribe anidado y la interfaz
        // pide `views.home.title`. Si esto se rompe, no hay una cadena que falle,
        // fallan todas.
        let v = json(r#"{"views":{"home":{"title":"Hola","sub":{"a":"A"}}},"suelta":"S"}"#);
        let plano = flatten_keys("", &v);
        assert_eq!(plano.get("views.home.title").map(String::as_str), Some("Hola"));
        assert_eq!(plano.get("views.home.sub.a").map(String::as_str), Some("A"));
        assert_eq!(plano.get("suelta").map(String::as_str), Some("S"));
        assert_eq!(plano.len(), 3);
    }

    #[test]
    fn un_valor_nulo_es_una_cadena_vacia_a_proposito() {
        // `clave:` sin valor en YAML es un texto vacío intencional, no un error.
        let plano = flatten_keys("", &json(r#"{"a":null}"#));
        assert_eq!(plano.get("a").map(String::as_str), Some(""));
    }

    #[test]
    fn un_arreglo_no_se_guarda_como_texto_vacio() {
        // Antes se guardaba como cadena vacía, así que la clave *existía* con
        // contenido vacío: la interfaz mostraba un hueco y no había forma de notar
        // que el catálogo estaba mal. Sin guardarla, la clave se ve cruda y alguien
        // la arregla.
        let plano = flatten_keys("", &json(r#"{"lista":["a","b"],"buena":"texto"}"#));
        assert!(!plano.contains_key("lista"), "{plano:?}");
        assert_eq!(plano.get("buena").map(String::as_str), Some("texto"));
    }

    #[test]
    fn los_numeros_y_booleanos_se_vuelven_texto() {
        // Un `1` o un `true` en un catálogo es un descuido, pero mostrarlo es mejor
        // que perder la clave.
        let plano = flatten_keys("", &json(r#"{"n":42,"b":true}"#));
        assert_eq!(plano.get("n").map(String::as_str), Some("42"));
        assert_eq!(plano.get("b").map(String::as_str), Some("true"));
    }

    #[test]
    fn el_prefijo_se_respeta() {
        let plano = flatten_keys("raiz", &json(r#"{"a":"A"}"#));
        assert_eq!(plano.get("raiz.a").map(String::as_str), Some("A"));
    }

    #[test]
    fn se_leen_los_tres_formatos() {
        assert!(parse_file("a: Hola\n", "yml", "es").is_ok());
        assert!(parse_file(r#"{"a":"Hola"}"#, "json", "es").is_ok());
        assert!(parse_file("a = \"Hola\"\n", "toml", "es").is_ok());
    }

    #[test]
    fn una_extension_desconocida_se_rechaza_sin_pánico() {
        assert!(parse_file("a: Hola", "txt", "es").is_err());
    }

    #[test]
    fn un_yaml_mal_formado_devuelve_error_y_no_entra_en_panico() {
        // Con `: ` dentro de un valor sin comillas, el parser lo lee como un mapeo
        // anidado y rompe el archivo entero. Es el error más común al escribir
        // catálogos, y antes tumbaba la aplicación entera.
        let malo = "a: esto tiene: dos puntos\n";
        assert!(parse_file(malo, "yml", "es").is_err());
    }

    #[test]
    fn sin_version_se_asume_la_uno() {
        assert_eq!(get_version(&json("{}")), 1);
        assert_eq!(get_version(&json(r#"{"_version":2}"#)), 2);
        // Una versión que no es un número cae en 1 en lugar de romper.
        assert_eq!(get_version(&json(r#"{"_version":"dos"}"#)), 1);
    }

    #[test]
    fn el_formato_v1_usa_el_idioma_del_nombre_del_archivo() {
        let trs = parse_file("saludo: Hola\n", "yml", "es").expect("parsea");
        assert!(trs.contains_key("es"));
        assert_eq!(trs["es"]["saludo"], json(r#""Hola""#));
    }

    #[test]
    fn el_formato_v2_reparte_por_idioma() {
        let contenido = "_version: 2\nsaludo:\n  es: Hola\n  en: Hello\n";
        let trs = parse_file(contenido, "yml", "es").expect("parsea");
        assert_eq!(flatten_keys("", &trs["es"]).get("saludo").map(String::as_str), Some("Hola"));
        assert_eq!(flatten_keys("", &trs["en"]).get("saludo").map(String::as_str), Some("Hello"));
    }

    #[test]
    fn juntar_dos_catalogos_no_pierde_las_claves_del_primero() {
        let mut a = json(r#"{"vista":{"titulo":"T","sub":"S"}}"#);
        merge_value(&mut a, &json(r#"{"vista":{"sub":"S2"},"otra":"O"}"#));
        let plano = flatten_keys("", &a);
        assert_eq!(plano.get("vista.titulo").map(String::as_str), Some("T"), "no se perdió");
        assert_eq!(plano.get("vista.sub").map(String::as_str), Some("S2"), "el nuevo gana");
        assert_eq!(plano.get("otra").map(String::as_str), Some("O"));
    }

    #[test]
    fn las_claves_se_unen_sin_puntos_de_sobra() {
        assert_eq!(format_keys(&["", "a"]), "a");
        assert_eq!(format_keys(&["a", "b"]), "a.b");
        assert_eq!(format_keys(&["", ""]), "");
    }

    #[test]
    fn un_catalogo_roto_no_se_lleva_a_los_demas() {
        // El arreglo que importa: esto entraba en pánico y **la aplicación no
        // abría** — pantalla negra, sin ventana y sin explicación. Un catálogo
        // roto tiene que degradar los textos, no impedir usar el programa.
        let base = con_catalogos(
            "roto",
            &[
                ("es.yml", "saludo: Hola\n"),
                ("en.yml", "esto: no es: yaml válido\n"),
            ],
        );

        let datos = load_data(Some(base.to_str().unwrap()));
        assert_eq!(
            datos.get("es").and_then(|m| m.get("saludo")).map(String::as_str),
            Some("Hola"),
            "el catálogo bueno tiene que cargar igual"
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn un_archivo_que_no_es_utf8_se_saltea() {
        let base = std::env::temp_dir().join(format!("i18n-cat-{}-binario", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        std::fs::write(base.join("es.yml"), "saludo: Hola\n").unwrap();
        std::fs::write(base.join("en.yml"), [0xff, 0xfe, 0x00, 0x01]).unwrap();

        let datos = load_data(Some(base.to_str().unwrap()));
        assert!(datos.contains_key("es"), "el bueno sigue");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn un_directorio_que_no_existe_deja_lo_empaquetado() {
        // Pasa cuando la ruta se resuelve mal, que es lo más común: la aplicación
        // tiene que abrir con los catálogos del binario, no morirse.
        let datos = load_data(Some("/no/existe/en/ninguna/parte"));
        assert_eq!(datos, load_data(None));
    }

    #[test]
    fn el_catalogo_de_ejecucion_le_gana_clave_por_clave_al_empaquetado() {
        // Es la razón de ser del `runtime_path`: se cambia una cadena en
        // `/usr/share` sin recompilar, y las que no se tocaron siguen viniendo del
        // binario.
        let base = con_catalogos("override", &[("es.yml", "uno: DESDE-DISCO\n")]);

        let datos = load_data(Some(base.to_str().unwrap()));
        assert_eq!(
            datos.get("es").and_then(|m| m.get("uno")).map(String::as_str),
            Some("DESDE-DISCO")
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn un_nombre_de_archivo_sin_idioma_se_saltea() {
        // `.yml` sin nada delante dejaba el idioma como cadena vacía, y las claves
        // quedaban bajo un idioma llamado «» que nadie puede seleccionar.
        let base = con_catalogos("sin-idioma", &[(".yml", "a: b\n"), ("es.yml", "c: d\n")]);
        let datos = load_data(Some(base.to_str().unwrap()));
        assert!(!datos.contains_key(""), "{:?}", datos.keys().collect::<Vec<_>>());
        assert!(datos.contains_key("es"));
        let _ = std::fs::remove_dir_all(&base);
    }
}
