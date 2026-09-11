import { beforeAll, describe, expect, mock, test } from 'bun:test';
import { crearBackend } from './backend-falso';

const backend = crearBackend();

mock.module('@tauri-apps/api/core', () => ({ invoke: backend.invoke }));
mock.module('@tauri-apps/api/event', () => ({ listen: backend.listen }));

const { default: I18n, useI18n } = await import('../guest-js/index');

/**
 * Dos cargas a la vez, y una suscripción que falla.
 *
 * Las dos cosas las encontró la revisión del PR y las dos son del mismo
 * descuido: dar por hecho que entre pedir algo al backend y recibirlo no pasa
 * nada más.
 */
describe('un reload encima de una carga que todavía no volvió', () => {
	beforeAll(() => {
		backend.reiniciarCuentas();
	});

	test('manda la última que se pidió, no la última que contesta', async () => {
		// La primera tarda; la segunda entra mientras tanto y contesta antes. Sin
		// el número de generación, la vieja terminaba última y pisaba los
		// catálogos nuevos con los viejos — o sea que releer dejaba lo de antes.
		backend.demorarLaProxima(60);
		const vieja = I18n.getInstance().load();

		await new Promise((listo) => setTimeout(listo, 10));
		backend.servirCatalogos({ es: { 'vistas.inicio.titulo': 'Inicio nuevo' } });
		const nueva = I18n.getInstance().reload();

		await Promise.all([vieja, nueva]);

		expect(useI18n().t('vistas.inicio.titulo')).toBe('Inicio nuevo');
	});
});

describe('si falla suscribirse al cambio de idioma', () => {
	beforeAll(() => {
		I18n.getInstance().destroy();
		backend.reiniciarCuentas();
		backend.servirCatalogos({ es: { 'vistas.inicio.titulo': 'Inicio' } });
	});

	test('la carga falla, pero se puede reintentar', async () => {
		// Antes, la marca de «ya escucho» se ponía **antes** de que `listen()`
		// volviera y el fallo quedaba fuera de la limpieza: `load()` se quedaba
		// con la promesa rechazada y la suscripción por hecha, así que cada
		// llamada posterior devolvía el mismo rechazo y nadie escuchaba nunca el
		// cambio de idioma. Un tropiezo de una vez rompía los textos para toda la
		// sesión.
		backend.hacerFallarElListen(new Error('no se pudo suscribir'));
		await expect(I18n.getInstance().load()).rejects.toThrow('no se pudo suscribir');

		await I18n.getInstance().load();
		expect(useI18n().t('vistas.inicio.titulo')).toBe('Inicio');
		expect(backend.veces('listen:i18n:locale_changed')).toBe(2);
	});

	test('y después el cambio de idioma llega igual', () => {
		backend.servirCatalogos({ es: { a: 'a' }, en: { a: 'b' } });
		backend.cambiarIdiomaDesdeAfuera('en');
		expect(useI18n().locale.value).toBe('en');
	});
});
