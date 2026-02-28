import { invoke } from '@tauri-apps/api/core'
import { listen, UnlistenFn } from '@tauri-apps/api/event';

type TranslationMap = Record<string, Record<string, string>>;


/**
 * **i18n**
 *
 * The `i18n` class serves as the primary interface for
 * communicating with the rust side of the plugin.
 * Default locale is en
 */
export default class I18n {
  private translations: TranslationMap | null = null;
  private locale = "en";
  private readonly elements = new Map<HTMLElement, string>();
  private unlistenFns: UnlistenFn[] = [];

  private static instance: I18n;

  private constructor() { } // private for singleton

  static getInstance(): I18n {
    if (!I18n.instance) I18n.instance = new I18n();
    return I18n.instance;
  }


  /** Load translations and setup listener */
  async load(): Promise<void> {
    this.translations = await invoke<Record<string, Record<string, string>> | null>('plugin:i18n|load_translations');
    this.locale = await invoke<string>('plugin:i18n|get_locale');

    const unlisten = await listen<string>('i18n:locale_changed', (event) => {
      this.locale = event.payload;
      this.updateAll();
    })

    this.unlistenFns.push(unlisten)
  }


  translate(key: string): string {
    if (!this.translations || !this.translations[this.locale]) {
      return key; // Return key as fallback
    }
    return this.translations[this.locale][key] ?? key;
  }


  /** Bind a single element to a key */
  bind(el: HTMLElement, key: string) {
    this.elements.set(el, key);
    el.textContent = this.translate(key);
  }

  /** Internal: updates all bound elements */
  private updateAll() {
    for (const [el, key] of this.elements.entries()) {
      el.textContent = this.translate(key);
    }
  }

  /**
   * **setLocale**
   * 
   * Sets the locale to the one passed in. eg: "zh-CN", "en-US"
   * @returns void
   * 
   * @example
   * ```ts
   * await i18n.setLocale("zh-CN");
   * ```
   */
  static async setLocale(locale: string): Promise<void> {
    await invoke<void>('plugin:i18n|set_locale', {
      locale: locale
    })
  }

  /**
   * **getLocale**
   * 
   * Gets the currently active locale. eg: "zh-CN", "en-US"
   * @returns string
   * 
   * @example
   * ```ts
   * await i18n.getLocale();
   * ```
   */
  static async getLocale(): Promise<string> {
    const locale = await invoke<string>('plugin:i18n|get_locale');
    return locale;
  }

  /**
   * **getAvailableLocale**
   * 
   * Gets all the available locale. eg: "zh-CN", "en-US"
   * @returns string[]
   * 
   * @example
   * ```ts
   * await i18n.getAvailableLocales();
   * ```
   */
  static async getAvailableLocales(): Promise<string[]> {
    const locale = await invoke<string[]>('plugin:i18n|get_available_locales');
    return locale;
  }

  // Clean up when done
  destroy() {
    this.unlistenFns.forEach(unlisten => unlisten());
    this.unlistenFns = [];
  }

}

// Export Vue 3 composable from separate file
// (requires Vue as peer dependency)
export { useI18n } from './composable';