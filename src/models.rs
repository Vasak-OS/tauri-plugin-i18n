use std::{collections::BTreeMap, sync::Mutex};

use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::backend::load_data;

/// Toma el candado sin morirse si quedó envenenado.
///
/// Un pánico en cualquier parte mientras alguien tiene el candado del idioma lo
/// envenena. A partir de ahí `set_locale` entraba en pánico y `translate` devolvía
/// `None` para todo: la aplicación se queda mostrando las claves crudas para
/// siempre. El idioma es una cadena; recuperarla es mejor que perder los textos.
fn bloquear<T>(candado: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    candado.lock().unwrap_or_else(|envenenado| envenenado.into_inner())
}

#[derive(Debug)]
pub struct PluginI18n<R: Runtime> {
    pub app: AppHandle<R>,
    pub data: BTreeMap<String, BTreeMap<String, String>>,
    pub locale: Mutex<String>,
}

impl<R: Runtime> PluginI18n<R> {
    ///
    /// Initialize the data using the locale and optional locales path
    ///
    pub fn new(app: tauri::AppHandle<R>, locale: String, locales_path: Option<String>) -> Self {
        // Load data from runtime path if provided, otherwise use bundled data
        let data = load_data(locales_path.as_deref());
        Self {
            app,
            data,
            locale: Mutex::new(locale),
        }
    }

    ///
    /// Gets the available locales
    ///
    pub fn available_locales(&self) -> Vec<String> {
        self.data.keys().map(|k| k.to_string()).collect()
    }

    ///
    /// Gets the translated string according to the current locale
    ///
    pub fn translate(&self, key: &str) -> Option<&str> {
        let locale = bloquear(&self.locale);

        self.data
            .get(locale.as_str())?
            .get(key)
            .map(|k| k.as_str())
    }

    ///
    /// Returns the data used for translations
    ///
    pub fn get_translations_data(&self) -> BTreeMap<String, BTreeMap<String, String>> {
        self.data.clone()
    }

    ///
    /// Update the locale.
    /// eg: "zh-CN", "en-US"
    ///
    pub fn set_locale(&self, locale: &str) {
        {
            let mut l = bloquear(&self.locale);
            *l = locale.to_string();
        }
        // El aviso va **fuera** del candado: un escucha que vuelva a preguntar el
        // idioma desde el mismo hilo se quedaría trabado contra sí mismo.
        let _ = self.app.emit("i18n:locale_changed", locale);
    }

    ///
    /// Get the current locale.
    /// eg: "zh-CN", "en-US"
    /// Default locale is "en".
    ///
    pub fn get_locale(&self) -> String {
        bloquear(&self.locale).to_string()
    }
}

pub trait PluginI18nExt<R: Runtime> {
    fn i18n(&self) -> &PluginI18n<R>;
}

impl<R: Runtime, T: Manager<R>> PluginI18nExt<R> for T {
    fn i18n(&self) -> &PluginI18n<R> {
        self.state::<PluginI18n<R>>().inner()
    }
}
