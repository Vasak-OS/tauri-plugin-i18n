//! Dónde buscar los catálogos de idioma.
//!
//! Es la parte que se rompe callado: si no se encuentra el directorio, la
//! aplicación abre igual y muestra las claves crudas —`views.home.title`— en lugar
//! de los textos. Pasó en el gestor de archivos, en la terminal y en la galería.
//!
//! La causa era que sólo se probaban rutas relativas al ejecutable y al directorio
//! de trabajo, y **ninguna existe cuando el binario está instalado en `/usr/bin`**.
//! Cada aplicación terminó pasando la ruta a mano. Con `/usr/share/<paquete>/locales`
//! en la lista, el caso instalado funciona solo.

use std::path::{Path, PathBuf};

/// Prefijos donde vive una aplicación instalada, y no una compilada al momento.
const PREFIJOS_DEL_SISTEMA: [&str; 3] = ["/usr/", "/opt/", "/snap/"];

/// Si este ejecutable es el que instaló un paquete.
///
/// Decide si las rutas relativas al directorio actual tienen algo que decir. Un
/// binario en `/usr/bin` trae sus catálogos en `/usr/share`, y no hay ningún
/// caso legítimo en que deba cargar los de donde alguien lo haya ejecutado.
fn esta_instalado(exe: &Path) -> bool {
    let ruta = exe.to_string_lossy();
    PREFIJOS_DEL_SISTEMA
        .iter()
        .any(|prefijo| ruta.starts_with(prefijo))
}

/// Los lugares donde puede estar el directorio de catálogos, en orden.
///
/// Primero los de desarrollo: con un binario compilado dentro del proyecto, sus
/// propios catálogos tienen que ganarle a los que haya instalados en el sistema, o
/// se prueba un cambio de traducción y no se ve.
///
/// Pero **sólo** si el binario no está instalado. Un proceso hereda el directorio
/// de trabajo de quien lo lanzó, y el escritorio abre las aplicaciones desde el
/// suyo: con el escritorio arrancado a mano desde el árbol de fuentes de otra
/// aplicación Tauri, toda aplicación abierta desde el menú encontraba
/// `<ese directorio>/locales` —los catálogos del escritorio— antes que los
/// propios, no reconocía ninguna clave y mostraba la interfaz en crudo:
/// `app.titulo`, `recursos.cpu`, `ajustes.intervalo`. Pasó con el monitor, con
/// las capturas y con la configuración, y el mismo binario abierto desde otro
/// lado andaba perfecto, que es lo que hacía difícil de creer el informe.
pub fn candidate_paths(exe: Option<&Path>, cwd: Option<&Path>) -> Vec<PathBuf> {
    let mut rutas = Vec::new();

    let instalado = exe.is_some_and(esta_instalado);

    if let Some(cwd) = cwd.filter(|_| !instalado) {
        rutas.push(cwd.join("locales"));
        rutas.push(cwd.join("src-tauri/locales"));
    }

    // `Path::new("app").parent()` devuelve `Some("")`, no `None`: sin descartarlo,
    // las rutas quedan relativas —`../locales`— y se cargaría cualquier `locales/`
    // que hubiera al lado de donde se ejecutó el proceso.
    if let Some(dir) = exe
        .and_then(|e| e.parent())
        .filter(|d| !d.as_os_str().is_empty())
    {
        rutas.push(dir.join("../locales"));
        rutas.push(dir.join("../../locales"));

        // El caso instalado: `/usr/bin/app` -> `/usr/share/app/locales`. Es la
        // convención de todos los paquetes de VasakOS.
        if let Some(nombre) = exe.and_then(|e| e.file_name()).and_then(|n| n.to_str()) {
            rutas.push(dir.join(format!("../share/{nombre}/locales")));
        }

        rutas.push(dir.join("../../../../Bundle/Resources/locales"));
    }

    rutas
}

/// La primera que exista y sea un directorio.
pub fn first_existing(rutas: &[PathBuf]) -> Option<PathBuf> {
    rutas.iter().find(|r| r.is_dir()).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_caso_instalado_llega_a_usr_share() {
        // Esto es lo que faltaba: con el binario en `/usr/bin`, ninguna de las
        // rutas relativas existía y la aplicación mostraba las claves crudas.
        let rutas = candidate_paths(Some(Path::new("/usr/bin/vasak-terminal")), None);
        assert!(
            rutas.contains(&PathBuf::from("/usr/bin/../share/vasak-terminal/locales")),
            "{rutas:?}"
        );
    }

    #[test]
    fn el_desarrollo_le_gana_al_sistema() {
        // Con un binario compilado dentro del proyecto, sus catálogos tienen que
        // ganar: si no, se cambia una traducción y no se ve el cambio.
        let rutas = candidate_paths(
            Some(Path::new("/proyecto/src-tauri/target/release/app")),
            Some(Path::new("/proyecto")),
        );
        let primera = rutas.first().expect("hay rutas");
        assert_eq!(primera, &PathBuf::from("/proyecto/locales"));

        let indice_de = |aguja: &str| rutas.iter().position(|r| r.to_string_lossy().contains(aguja));
        assert!(indice_de("/proyecto/locales") < indice_de("../share/"));
    }

    #[test]
    fn una_aplicacion_instalada_no_mira_el_directorio_actual() {
        // La regresión: el escritorio abre las aplicaciones desde su propio
        // directorio de trabajo. Con el escritorio corriendo desde el árbol de
        // fuentes de otra aplicación Tauri, `<ese directorio>/locales` existe y
        // son los catálogos de otra aplicación: la que se abría cargaba esos, no
        // reconocía ninguna clave y mostraba `app.titulo` en vez del título.
        let rutas = candidate_paths(
            Some(Path::new("/usr/bin/vasak-monitor")),
            Some(Path::new("/home/pato/VasakOS/vasak-desktop/src-tauri")),
        );

        assert!(
            !rutas.iter().any(|r| r.starts_with("/home")),
            "una aplicación instalada no puede cargar catálogos del directorio actual: {rutas:?}"
        );
        assert!(
            rutas.contains(&PathBuf::from("/usr/bin/../share/vasak-monitor/locales")),
            "{rutas:?}"
        );
    }

    #[test]
    fn los_prefijos_del_sistema_son_los_que_cuentan() {
        assert!(esta_instalado(Path::new("/usr/bin/vasak-terminal")));
        assert!(esta_instalado(Path::new("/usr/local/bin/vasak-terminal")));
        assert!(esta_instalado(Path::new("/opt/cosa/bin/app")));
        // Un binario compilado sigue sin estarlo, aunque el proyecto viva en un
        // directorio con un nombre parecido.
        assert!(!esta_instalado(Path::new("/home/pato/proyecto/target/release/app")));
        assert!(!esta_instalado(Path::new("/home/usr/proyecto/target/release/app")));
    }

    #[test]
    fn sin_entorno_no_se_inventan_rutas() {
        // Sin ejecutable ni directorio de trabajo no hay nada que probar, y
        // devolver rutas relativas sueltas haría que se cargara cualquier
        // `locales/` que hubiera donde se ejecute.
        assert!(candidate_paths(None, None).is_empty());
    }

    #[test]
    fn un_ejecutable_sin_directorio_no_agrega_nada() {
        assert!(candidate_paths(Some(Path::new("app")), None).is_empty());
    }

    #[test]
    fn se_elige_el_primero_que_existe_y_es_directorio() {
        let base = std::env::temp_dir().join(format!("i18n-prueba-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("segundo")).unwrap();
        std::fs::write(base.join("archivo"), b"no soy un directorio").unwrap();

        let rutas = vec![
            base.join("no-existe"),
            base.join("archivo"),
            base.join("segundo"),
        ];
        assert_eq!(first_existing(&rutas), Some(base.join("segundo")));

        assert_eq!(first_existing(&[base.join("nada")]), None);
        assert_eq!(first_existing(&[]), None);
        let _ = std::fs::remove_dir_all(&base);
    }
}
