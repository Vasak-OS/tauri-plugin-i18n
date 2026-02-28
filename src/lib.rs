use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};

pub use models::*;

mod backend;
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
            
            // Try to find locales directory
            let found_locales_path = locales_path.clone().or_else(|| {
                // Get app directory
                if let Some(app_dir) = app.path().app_config_dir().ok() {
                    // In dev mode try ../../src-tauri/locales (from target/debug/...)
                    // In release mode try ../locales
                    let potential_paths = vec![
                        app_dir.join("../../src-tauri/locales"),
                        app_dir.join("../../../src-tauri/locales"),
                        app_dir.join("../locales"),
                        app_dir.join("locales"),
                    ];
                    
                    for path in potential_paths {
                        if path.exists() && path.is_dir() {
                            println!("[i18n] Found locales at: {}", path.display());
                            return Some(path.to_string_lossy().to_string());
                        }
                    }
                }
                
                println!("[i18n] Warning: Could not find locales directory, using empty translations");
                None
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
