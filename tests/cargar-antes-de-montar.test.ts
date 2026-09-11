import { beforeAll, describe, expect, mock, test } from 'bun:test';
import { crearBackend } from './backend-falso';

const backend = crearBackend();

mock.module('@tauri-apps/api/core', () => ({ invoke: backend.invoke }));
mock.module('@tauri-apps/api/event', () => ({ listen: backend.listen }));

const { default: I18n, useI18n } = await import('../guest-js/index');

/**
 * **El orden que usan las ocho aplicaciones del escritorio.**
 *
 * Va en un archivo aparte, y el `test` del `package.json` corre cada archivo en
 * su propio proceso, porque el estado del plugin es un singleton y esto necesita
 * el momento exacto en que nadie llamó todavía a `useI18n()`: cargar
 * **primero** y recién después pedir el composable, que es lo que hace un
 * `main.ts` que quiere los textos puestos antes de montar.
 *
 * Es el caso que más importa y el más fácil de romper sin darse cuenta: la
 * suscripción al estado de la clase se registra en el primer `useI18n()`, o sea
 * después de que el aviso de la carga ya pasó. Si al suscribirse no se lee
 * además el estado que ya está, estas referencias se quedan vacías hasta el
 * próximo cambio —que puede no llegar nunca— y la ventana muestra las claves
 * crudas igual que antes del arreglo.
 */
describe('cargar antes de montar', () => {
	// `mock.module` vuelve a evaluar el plugin en cada archivo de pruebas, así que
	// la clase nace limpia acá; el backend de mentira, en cambio, se comparte entre
	// archivos y hay que ponerle las cuentas a cero.
	beforeAll(() => {
		backend.reiniciarCuentas();
	});

	test('el primer useI18n() posterior a la carga encuentra los textos puestos', async () => {
		// La condición de la que depende todo lo demás, comprobada por la clase y
		// no por `useI18n()` —que al llamarlo ya haría lo que se quiere probar—.
		// Si falla, este archivo corrió compartiendo el módulo con otro y lo de
		// abajo pasaría sin probar nada.
		expect(I18n.getInstance().catalogs).toBeNull();

		// Esto es `main.ts`, antes de `app.mount('#app')`.
		await I18n.getInstance().load();

		// Y esto es el primer componente que se monta.
		const { t, isLoaded, locale } = useI18n();

		expect(isLoaded.value).toBe(true);
		expect(locale.value).toBe('es');
		expect(t('vistas.inicio.titulo')).toBe('Inicio');
	});

	test('y no hizo falta una segunda carga', () => {
		expect(backend.veces('plugin:i18n|load_translations')).toBe(1);
	});
});
