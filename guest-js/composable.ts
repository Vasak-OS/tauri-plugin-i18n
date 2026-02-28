import { ref, computed, onMounted } from 'vue';
import I18n from './index';

type TranslationMap = Record<string, Record<string, string>>;

// Singleton state - shared across all component instances
const translations = ref<TranslationMap | null>(null);
const locale = ref<string>('en');
const isLoaded = ref(false);

let i18nInstance: I18n | null = null;

/**
 * Vue 3 Composable for internationalization with reactive `t()` function
 * 
 * @example
 * ```vue
 * <script setup>
 * import { useI18n } from '@vasakgroup/tauri-plugin-i18n/composable'
 * 
 * const { t, locale, setLocale, availableLocales } = useI18n()
 * </script>
 * 
 * <template>
 *   <p>{{ t('settings.navigator.navigatorOptions') }}</p>
 *   <button @click="setLocale('es')">Español</button>
 * </template>
 * ```
 */
export function useI18n() {
  // Initialize on first use
  onMounted(async () => {
    if (!isLoaded.value) {
      await loadTranslations();
    }
  });

  /**
   * Translate a key to current locale
   * Returns the key if translation not found
   */
  const t = computed(() => {
    return (key: string): string => {
      if (!translations.value || !translations.value[locale.value]) {
        return key;
      }
      return translations.value[locale.value][key] ?? key;
    };
  });

  /**
   * Get available locales
   */
  const availableLocales = computed(() => {
    if (!translations.value) return [];
    return Object.keys(translations.value);
  });

  /**
   * Set the current locale
   */
  async function setLocale(newLocale: string) {
    await I18n.setLocale(newLocale);
    locale.value = newLocale;
  }

  /**
   * Load translations from backend
   */
  async function loadTranslations() {
    try {
      i18nInstance ??= I18n.getInstance();
      await i18nInstance.load();
      
      // Get data from instance
      translations.value = (i18nInstance as any).translations;
      locale.value = (i18nInstance as any).locale;
      isLoaded.value = true;

      console.log('[useI18n] Loaded translations:', {
        locales: availableLocales.value,
        currentLocale: locale.value
      });
    } catch (error) {
      console.error('[useI18n] Failed to load translations:', error);
    }
  }

  return {
    t: t.value,
    locale: computed(() => locale.value),
    setLocale,
    availableLocales,
    isLoaded: computed(() => isLoaded.value),
    reload: loadTranslations
  };
}
