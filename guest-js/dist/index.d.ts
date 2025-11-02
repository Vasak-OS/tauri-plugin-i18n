/**
 * **i18n**
 *
 * The `i18n` class serves as the primary interface for
 * communicating with the rust side of the plugin.
 * Default locale is en
 */
export default class I18n {
    private static _instance;
    private translations;
    private locale;
    private elements;
    private observer;
    private unlistenFns;
    private bindings;
    private static instance;
    private constructor();
    static getInstance(): I18n;
    /** Load translations and setup listener */
    load(): Promise<void>;
    translate(key: string): string;
    /** Bind a single element to a key */
    bind(el: HTMLElement, key: string): void;
    /** Find and bind all elements with [data-i18n] */
    autoBind(): void;
    /** Observe DOM for new [data-i18n] elements */
    private observeDOM;
    /** Internal: updates all bound elements */
    private updateAll;
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
    static setLocale(locale: string): Promise<void>;
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
    static getLocale(): Promise<string>;
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
    static getAvailableLocales(): Promise<string[]>;
    destroy(): void;
}
//# sourceMappingURL=index.d.ts.map