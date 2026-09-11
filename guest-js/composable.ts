import { ref, computed, getCurrentInstance, onMounted } from 'vue';
import I18n from './index';

type TranslationMap = Record<string, Record<string, string>>;

/**
 * El estado que lee la interfaz, atado al de la clase.
 *
 * **No es una copia.** Antes sí lo era —estas mismas referencias, pero llenadas
 * sólo por `loadTranslations()`— y eso hacía que esperar a
 * `I18n.getInstance().load()` antes de montar no sirviera de nada: la clase
 * quedaba cargada, estas referencias vacías, y el `t()` de los componentes
 * —que lee de acá— devolvía la clave cruda hasta que el `onMounted` del primer
 * componente volvía a cargar todo por su cuenta. Se veía como unos segundos de
 * «views.home.title» en la ventana recién abierta.
 *
 * Ahora la clase es la dueña del estado y esto lo refleja, así que llenarlo por
 * cualquiera de los dos caminos lo llena para los dos.
 */
const translations = ref<TranslationMap | null>(null);
const locale = ref<string>('en');
const isLoaded = ref(false);

let suscripto = false;

/**
 * Ata estas referencias al estado de la clase, la primera vez que hace falta.
 *
 * Va acá dentro y no en el cuerpo del módulo por un detalle de orden: `index.ts`
 * reexporta `useI18n`, así que este archivo se evalúa **antes** de que la clase
 * exista. Tocar `I18n` al importar revienta con «Cannot access 'I18n' before
 * initialization», y como el paquete se publica en un solo archivo empaquetado,
 * es un error que aparecería recién en la aplicación.
 *
 * La suscripción es una sola para todo el módulo y no se da de baja: la clase es
 * un singleton y estas referencias también, así que vivir lo que viva la ventana
 * es exactamente lo que corresponde.
 */
function atarAlEstadoDeLaClase() {
  const instancia = I18n.getInstance();
  const reflejar = () => {
    translations.value = instancia.catalogs;
    locale.value = instancia.currentLocale;
    isLoaded.value = instancia.catalogs !== null;
  };
  if (!suscripto) {
    suscripto = true;
    instancia.subscribe(reflejar);
  }
  // Y se refleja ya, que es lo que hace que un `useI18n()` posterior a la carga
  // encuentre los textos puestos en vez de esperar al próximo aviso.
  reflejar();
  return instancia;
}

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
  const instancia = atarAlEstadoDeLaClase();

  // Initialize on first use
  //
  // Sólo dentro de un componente. Llamar a `useI18n()` desde `main.ts` —para
  // tener los textos antes de montar, que es lo que conviene hacer— es
  // perfectamente válido, y ahí este `onMounted` no tiene a quién asociarse:
  // sin la guarda, Vue lo avisa por consola en cada arranque en desarrollo.
  if (getCurrentInstance()) {
    onMounted(() => {
      if (!isLoaded.value) void asegurarCargados();
    });
  }

  /**
   * Translate a key to current locale
   * Returns the key if translation not found
   */
  const t = (key: string): string => {
    // Las dos lecturas de `.value` son las que atan el renderizado a este
    // estado: cuando el catálogo llega, lo que use `t()` se vuelve a dibujar.
    if (!translations.value || !translations.value[locale.value]) {
      return key;
    }
    return translations.value[locale.value][key] ?? key;
  };

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
  }

  /**
   * Se asegura de que los catálogos estén cargados, sin volver a pedirlos.
   *
   * Se puede llamar de a muchos: la clase hace una sola carga y los demás
   * esperan a esa. Antes cada componente hacía la suya.
   */
  async function asegurarCargados() {
    try {
      await instancia.load();
    } catch (error) {
      console.error('[useI18n] Failed to load translations:', error);
    }
  }

  /**
   * Load translations from backend
   *
   * Los relee aunque ya estén: es lo que hace falta cuando los archivos de
   * idioma cambiaron en el disco. Para tenerlos antes de montar —el uso
   * habitual desde `main.ts`— alcanza con esto igual, porque la primera vez no
   * hay nada que releer.
   */
  async function reload() {
    try {
      await instancia.reload();
    } catch (error) {
      console.error('[useI18n] Failed to load translations:', error);
    }
  }

  return {
    t,
    locale: computed(() => locale.value),
    setLocale,
    availableLocales,
    isLoaded: computed(() => isLoaded.value),
    reload
  };
}
