/**
 * Un backend de mentira, con la cuenta de cuántas veces lo llamaron.
 *
 * La cuenta es la mitad de lo que se prueba acá: lo que estas pruebas cuidan no
 * es sólo que los textos lleguen, sino **cuántas veces** se piden.
 *
 * Es una fábrica y no un módulo con estado: `mock.module` hace que el plugin se
 * vuelva a evaluar en cada archivo de pruebas, y con estado compartido las
 * cuentas de un archivo aparecían en otro —o no aparecían en ninguno—, así que
 * la prueba pasaba o fallaba según el orden. Cada archivo se arma el suyo.
 */
export const CATALOGOS: Record<string, Record<string, string>> = {
	es: { 'vistas.inicio.titulo': 'Inicio' },
	en: { 'vistas.inicio.titulo': 'Home' },
};

export function crearBackend() {
	const llamadas: Record<string, number> = {};
	let idioma = 'es';
	let fallaLaProxima: Error | null = null;
	/** Lo que el backend escucha en `i18n:locale_changed`, para dispararlo a mano. */
	let avisar: ((locale: string) => void) | null = null;

	const contar = (comando: string) => {
		llamadas[comando] = (llamadas[comando] ?? 0) + 1;
	};

	const invoke = async (comando: string, args?: Record<string, unknown>) => {
		contar(comando);
		switch (comando) {
			case 'plugin:i18n|load_translations': {
				if (fallaLaProxima) {
					const e = fallaLaProxima;
					fallaLaProxima = null;
					throw e;
				}
				return CATALOGOS;
			}
			case 'plugin:i18n|get_locale':
				return idioma;
			case 'plugin:i18n|set_locale':
				idioma = args?.locale as string;
				return null;
			case 'plugin:i18n|get_available_locales':
				return Object.keys(CATALOGOS);
			default:
				return null;
		}
	};

	const listen = async (evento: string, manejador: (e: { payload: string }) => void) => {
		contar(`listen:${evento}`);
		if (evento === 'i18n:locale_changed') {
			avisar = (locale: string) => manejador({ payload: locale });
		}
		return () => {
			contar(`unlisten:${evento}`);
		};
	};

	return {
		invoke,
		listen,
		llamadas,
		/** Cuántas veces se llamó a un comando; cero si nunca. */
		veces: (comando: string) => llamadas[comando] ?? 0,
		reiniciarCuentas: () => {
			for (const k of Object.keys(llamadas)) delete llamadas[k];
		},
		hacerFallar: (error: Error) => {
			fallaLaProxima = error;
		},
		/** Como si alguien cambiara el idioma desde otra ventana. */
		cambiarIdiomaDesdeAfuera: (locale: string) => avisar?.(locale),
	};
}
