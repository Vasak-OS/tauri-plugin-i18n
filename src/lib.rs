use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};

pub use models::*;

mod backend;
mod discovery;
mod commands;
mod error;
mod models;

pub use error::{Error, Result};

/// Initializes the plugin.
/// 
/// # Arguments
/// 
/// * `locale` - Optional default locale (e.g., "en", "es"). Defaults to "en".
/// * `locales_path` - Optional path to locales directory. If None, uses "src-tauri/locales" or "../locales".
pub fn init<R: Runtime>(locale: Option<String>) -> TauriPlugin<R> {
    init_with_path(locale, None)
}

/// Initializes the plugin with a custom locales path.
pub fn init_with_path<R: Runtime>(locale: Option<String>, locales_path: Option<String>) -> TauriPlugin<R> {
    Builder::new("i18n")
        .invoke_handler(tauri::generate_handler![
            commands::load_translations,
            commands::translate,
            commands::set_locale,
            commands::get_locale,
            commands::get_available_locales,
        ])
        .setup(move |app, _api| {
            let default_locale = locale.clone().unwrap_or("en".to_string());
            
            // Dónde están los catálogos. Ver `discovery`: es la parte que se
            // rompe callado, porque sin ella la aplicación abre igual y muestra
            // las claves crudas en lugar de los textos.
            let found_locales_path = locales_path.clone().or_else(|| {
                let exe = std::env::current_exe().ok();
                let cwd = std::env::current_dir().ok();
                let candidatas = discovery::candidate_paths(exe.as_deref(), cwd.as_deref());

                match discovery::first_existing(&candidatas) {
                    Some(ruta) => Some(ruta.to_string_lossy().to_string()),
                    None => {
                        // Se avisa sólo cuando no se encontró, que es el caso en
                        // que hace falta: los textos van a salir como claves y sin
                        // esto no hay ninguna pista de por qué.
                        eprintln!(
                            "[i18n] no se encontró el directorio de catálogos; se usan los \
                             empaquetados. Se buscó en: {}",
                            candidatas
                                .iter()
                                .map(|r| r.display().to_string())
                                .collect::<Vec<_>>()
                                .join(", ")
                        );
                        None
                    }
                }
            });

            app.manage(PluginI18n::new(
                app.clone(),
                default_locale,
                found_locales_path,
            ));

            Ok(())
        })
        .build()
}
