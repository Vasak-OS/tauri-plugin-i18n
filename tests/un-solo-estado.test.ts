import { beforeAll, describe, expect, mock, test } from 'bun:test';
import { crearBackend } from './backend-falso';

const backend = crearBackend();

mock.module('@tauri-apps/api/core', () => ({ invoke: backend.invoke }));
mock.module('@tauri-apps/api/event', () => ({ listen: backend.listen }));

const { default: I18n, useI18n } = await import('../guest-js/index');

/**
 * **El arreglo, en una prueba.**
 *
 * El plugin tenía dos estados con el mismo catálogo: el de la clase y el del
 * composable. El `t()` de los componentes lee el del composable, y cargar la
 * clase no lo tocaba. Así que una aplicación que hacía
 *
 *     await I18n.getInstance().load()
 *     app.mount('#app')
 *
 * —que es lo que hay que hacer, y lo que hacen las ocho del escritorio— se
 * montaba igual con el composable vacío: la ventana mostraba «vistas.inicio.titulo»
 * en lugar de «Inicio» hasta que el `onMounted` del primer componente volvía a
 * cargar todo por su cuenta.
 *
 * Esta prueba va primera **a propósito**: es la única que necesita la instancia
 * recién nacida, sin cargar.
 */
describe('la clase y el composable son un solo estado', () => {
	test('antes de cargar, t() devuelve la clave', () => {
		expect(useI18n().isLoaded.value).toBe(false);
		expect(useI18n().t('vistas.inicio.titulo')).toBe('vistas.inicio.titulo');
	});

	test('cargando la clase, el t() del composable ya traduce', async () => {
		await I18n.getInstance().load();

		const { t, isLoaded, locale, availableLocales } = useI18n();
		expect(isLoaded.value).toBe(true);
		expect(locale.value).toBe('es');
		expect(t('vistas.inicio.titulo')).toBe('Inicio');
		expect(availableLocales.value.sort()).toEqual(['en', 'es']);
	});

	test('una clave que no está se sigue viendo cruda', () => {
		// El respaldo no cambia: una clave sin traducir tiene que verse, que es
		// lo que hace que alguien la agregue.
		expect(useI18n().t('no.existe')).toBe('no.existe');
	});
});

describe('una sola carga, aunque la pidan muchos', () => {
	beforeAll(() => {
		I18n.getInstance().destroy();
		backend.reiniciarCuentas();
	});

	test('cincuenta pedidos a la vez y el backend se llama una vez', async () => {
		// Es el caso real, y es lo que hacen cincuenta `onMounted` en el mismo
		// instante: `vasak-settings` tiene cincuenta y tres componentes con
		// `useI18n()`, y cada uno pedía su carga. Ninguno se enteraba de los demás
		// —el estado estaba vacío para todos— así que salían ciento cincuenta y
		// nueve llamadas al backend al abrir la ventana.
		await Promise.all(Array.from({ length: 50 }, () => I18n.getInstance().load()));

		expect(backend.veces('plugin:i18n|load_translations')).toBe(1);
		expect(backend.veces('plugin:i18n|get_locale')).toBe(1);
	});

	test('y el componente que monta después no vuelve a pedir nada', () => {
		// El `onMounted` del composable sólo carga si falta; cargado antes de
		// montar —que es lo que conviene hacer en `main.ts`— no dispara ninguna.
		expect(useI18n().isLoaded.value).toBe(true);
	});

	test('y se escucha el cambio de idioma una sola vez', () => {
		// Cada carga registraba su propio `listen`, y ninguno se cancelaba: con
		// cincuenta componentes, cada cambio de idioma corría cincuenta veces.
		expect(backend.veces('listen:i18n:locale_changed')).toBe(1);
	});

	test('reload() sí vuelve a pedirlos', async () => {
		await I18n.getInstance().reload();
		expect(backend.veces('plugin:i18n|load_translations')).toBe(2);
		// Pero no vuelve a suscribirse.
		expect(backend.veces('listen:i18n:locale_changed')).toBe(1);
	});
});

describe('un fallo no deja la carga marcada como hecha', () => {
	beforeAll(() => {
		I18n.getInstance().destroy();
		backend.reiniciarCuentas();
	});

	test('si el backend no contesta, se puede reintentar', async () => {
		// Sin esto, un backend que falló una vez no se reintenta nunca y la
		// aplicación se queda con las claves crudas para siempre.
		backend.hacerFallar(new Error('el backend dijo que no'));
		await expect(I18n.getInstance().load()).rejects.toThrow('el backend dijo que no');

		await I18n.getInstance().load();
		expect(useI18n().t('vistas.inicio.titulo')).toBe('Inicio');
		expect(backend.veces('plugin:i18n|load_translations')).toBe(2);
	});
});

describe('cambiar de idioma', () => {
	test('setLocale cambia lo que devuelve t()', async () => {
		await useI18n().setLocale('en');
		expect(useI18n().t('vistas.inicio.titulo')).toBe('Home');
		expect(useI18n().locale.value).toBe('en');
	});

	test('el aviso del backend también', () => {
		// El backend emite `i18n:locale_changed` cuando alguien cambia el idioma
		// desde otra ventana. Tiene que llegar al `t()` igual.
		backend.cambiarIdiomaDesdeAfuera('es');
		expect(useI18n().t('vistas.inicio.titulo')).toBe('Inicio');
	});

	test('el aviso repetido del idioma que ya está no hace nada', () => {
		backend.cambiarIdiomaDesdeAfuera('es');
		expect(useI18n().locale.value).toBe('es');
	});
});

describe('usarlo fuera de un componente', () => {
	test('no avisa por consola', () => {
		// Tener los textos antes de montar obliga a llamar a `useI18n()` desde
		// `main.ts`, donde no hay componente. Sin la guarda de
		// `getCurrentInstance()`, Vue avisa que su `onMounted` no tiene a quién
		// asociarse — en cada arranque, en cada aplicación.
		const avisos: unknown[][] = [];
		const original = console.warn;
		console.warn = (...args: unknown[]) => {
			avisos.push(args);
		};
		try {
			useI18n();
		} finally {
			console.warn = original;
		}
		expect(avisos).toEqual([]);
	});
});
