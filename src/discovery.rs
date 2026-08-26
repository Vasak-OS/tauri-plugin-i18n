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

/// Los lugares donde puede estar el directorio de catálogos, en orden.
///
/// Primero los de desarrollo: con un binario compilado dentro del proyecto, sus
/// propios catálogos tienen que ganarle a los que haya instalados en el sistema, o
/// se prueba un cambio de traducción y no se ve.
pub fn candidate_paths(exe: Option<&Path>, cwd: Option<&Path>) -> Vec<PathBuf> {
    let mut rutas = Vec::new();

    if let Some(cwd) = cwd {
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
