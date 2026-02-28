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
                let mut potential_paths = vec![];
                
                // Strategy 1: Try from current executable path (for packaged apps)
                if let Ok(exe_path) = std::env::current_exe() {
                    if let Some(exe_dir) = exe_path.parent() {
                        // From target/debug/ or release builds
                        potential_paths.push(exe_dir.join("../locales"));
                        potential_paths.push(exe_dir.join("../../locales"));
                        // From bundled apps
                        potential_paths.push(exe_dir.join("../../../../Bundle/Resources/locales"));
                    }
                }
                
                // Strategy 2: From working directory (dev mode)
                if let Ok(cwd) = std::env::current_dir() {
                    potential_paths.push(cwd.join("locales"));
                    potential_paths.push(cwd.join("src-tauri/locales"));
                }
                
                // Log all potential paths
                eprintln!("[i18n] Searching for locales in {} potential paths:", potential_paths.len());
                for (i, path) in potential_paths.iter().enumerate() {
                    let exists = path.exists();
                }
                
                for path in potential_paths {
                    if path.exists() && path.is_dir() {
                        return Some(path.to_string_lossy().to_string());
                    }
                }
                
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
