import { invoke } from '@tauri-apps/api/core'
import { listen, UnlistenFn } from '@tauri-apps/api/event';

type TranslationMap = Record<string, Record<string, string>>;

/** Lo que se llama cuando cambian los catálogos o el idioma. */
type Oyente = () => void;

/**
 * **i18n**
 *
 * The `i18n` class serves as the primary interface for
 * communicating with the rust side of the plugin.
 * Default locale is en
 *
 * Es **el único** lugar donde viven los catálogos y el idioma. El composable de
 * Vue no guarda una copia: se suscribe con `subscribe()` y refleja lo que hay
 * acá. Antes tenía su propio estado, y eso significaba que cargar la clase no le
 * daba nada a la interfaz — ver el comentario de `composable.ts`.
 */
export default class I18n {
  private translations: TranslationMap | null = null;
  private locale = "en";
  private readonly elements = new Map<HTMLElement, string>();
  private unlistenFns: UnlistenFn[] = [];

  /** Quienes quieren enterarse de que el estado cambió. */
  private readonly oyentes = new Set<Oyente>();

  /**
   * La carga en curso, para no hacer dos.
   *
   * Sin esto, cada componente que use `useI18n()` arranca la suya en su
   * `onMounted`: todos ven el estado vacío en el mismo instante y ninguno sabe
   * de los demás. En una aplicación con cincuenta componentes eso es un
   * centenar y medio de llamadas al backend y cincuenta suscripciones al evento
   * de cambio de idioma, todas al abrir la ventana.
   */
  private cargando: Promise<void> | null = null;

  /**
   * La suscripción al cambio de idioma, en curso o hecha.
   *
   * Es una promesa y no un booleano a propósito. Con un booleano puesto antes de
   * que `listen()` volviera, dos cargas a la vez registraban dos suscripciones —
   * ninguna veía a la otra— y, peor, una que fallara quedaba marcada como hecha:
   * de ahí en más nadie escuchaba el cambio de idioma y no había forma de
   * reintentarlo. Guardando la promesa, la segunda carga espera a la primera y
   * un fallo se olvida.
   */
  private suscripcion: Promise<UnlistenFn> | null = null;

  /**
   * Cuál es la carga vigente.
   *
   * Dos cargas encimadas —una que empezó y un `reload()` que llega antes de que
   * termine— pueden volver en cualquier orden. Sin esto, la vieja terminando
   * última pisaba los catálogos de la nueva con los de antes.
   */
  private generacion = 0;

  private static instance: I18n;

  private constructor() { } // private for singleton

  static getInstance(): I18n {
    if (!I18n.instance) I18n.instance = new I18n();
    return I18n.instance;
  }

  /** Los catálogos cargados, o `null` si todavía no se cargaron. */
  get catalogs(): TranslationMap | null {
    return this.translations;
  }

  /** El idioma activo. */
  get currentLocale(): string {
    return this.locale;
  }

  /**
   * Avisa cuando cambian los catálogos o el idioma.
   *
   * Devuelve cómo darse de baja. Es lo que le permite al composable de Vue
   * reflejar este estado en lugar de llevar el suyo.
   */
  subscribe(oyente: Oyente): () => void {
    this.oyentes.add(oyente);
    return () => {
      this.oyentes.delete(oyente);
    };
  }

  /** Se copia el conjunto: un oyente que se da de baja acá no rompe el recorrido. */
  private notificar() {
    for (const oyente of [...this.oyentes]) oyente();
  }

  /**
   * Load translations and setup listener
   *
   * Se puede llamar todas las veces que haga falta: la primera hace el trabajo y
   * las demás esperan a esa misma. Para releer los catálogos —no para
   * asegurarse de que estén— está `reload()`.
   */
  async load(): Promise<void> {
    this.cargando ??= this.cargar();
    return this.cargando;
  }

  /** Relee los catálogos aunque ya estén cargados. */
  async reload(): Promise<void> {
    this.cargando = null;
    return this.load();
  }

  private async cargar(): Promise<void> {
    const mia = ++this.generacion;

    let catalogos: TranslationMap | null;
    let idioma: string;
    try {
      catalogos = await invoke<Record<string, Record<string, string>> | null>('plugin:i18n|load_translations');
      idioma = await invoke<string>('plugin:i18n|get_locale');
      await this.escucharCambioDeIdioma();
    } catch (error) {
      // Un fallo no puede dejar la carga marcada como hecha: sin esto, un
      // backend que no contestó una vez no se reintenta nunca y la aplicación
      // se queda con las claves crudas para siempre.
      //
      // Se limpia sólo si la vigente sigue siendo ésta: si mientras tanto
      // entró un `reload()`, `cargando` ya apunta a la carga nueva y borrarlo
      // la dejaría huérfana.
      if (mia === this.generacion) this.cargando = null;
      throw error;
    }

    // Lo de una carga vieja que termina tarde se descarta: manda la última que
    // se pidió, no la última que contesta.
    if (mia !== this.generacion) return;

    this.translations = catalogos;
    this.locale = idioma;
    this.notificar();
  }

  /**
   * Se suscribe al cambio de idioma, una sola vez y reintentable.
   */
  private async escucharCambioDeIdioma(): Promise<void> {
    this.suscripcion ??= listen<string>('i18n:locale_changed', (event) => {
      this.aplicarIdioma(event.payload);
    });

    let unlisten: UnlistenFn;
    try {
      unlisten = await this.suscripcion;
    } catch (error) {
      this.suscripcion = null;
      throw error;
    }

    // Dos cargas encimadas esperan a la misma promesa y vuelven con la misma
    // función: se guarda una vez, o `destroy()` la llamaría de más.
    if (!this.unlistenFns.includes(unlisten)) this.unlistenFns.push(unlisten);
  }

  /** Deja el idioma puesto y avisa a todo el que mire. */
  private aplicarIdioma(locale: string) {
    if (this.locale === locale) return;
    this.locale = locale;
    this.updateAll();
    this.notificar();
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
    // El backend además emite `i18n:locale_changed`, pero ese aviso sólo llega
    // si ya se llamó a `load()`. Aplicarlo acá hace que cambiar el idioma
    // funcione igual antes de la primera carga, y el evento que llegue después
    // no hace nada porque ya es el idioma puesto.
    I18n.getInstance().aplicarIdioma(locale);
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
    this.suscripcion = null;
    // La carga también se olvida: si no, después de un `destroy()` la instancia
    // se cree cargada y nadie vuelve a suscribirse al cambio de idioma.
    this.cargando = null;
    // Los oyentes **no** se tiran. El del composable se registra una sola vez,
    // al importar el módulo; si `destroy()` lo borrara, una ventana que se
    // destruye y vuelve a cargar dejaría la interfaz sin enterarse de nada, y
    // no hay forma de volver a suscribirla desde afuera.
  }

}

// Export Vue 3 composable from separate file
// (requires Vue as peer dependency)
export { useI18n } from './composable';
