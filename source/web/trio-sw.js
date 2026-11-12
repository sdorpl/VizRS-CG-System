// Service worker Trio: uruchamia aplikację bez sieci (telefon z ikony), żeby operator zobaczył stan połączenia zamiast błędu przeglądarki.
// Zawsze najpierw sieć: po aktualizacji serwera nikt nie może grać starym interfejsem. Komend i stanu anteny nigdy nie zapisuje.
const CACHE = 'vizrs-trio-1';
const SHELL = ['/trio', '/ui.css', '/text-model.js', '/renderer.js', '/qr.js', '/ticker-model.js', '/ticker-items.js', '/color-picker.js', '/richtext.js',
  '/motion.js', '/field-rules.js', '/native.js', '/dialogs.js', '/cgpack.js', '/trio-icon.svg', '/trio.webmanifest'];

addEventListener('install', () => skipWaiting());
// pamięć dawnego Trio na telefonie (/mobile) nie jest już potrzebna
addEventListener('activate', (e) => e.waitUntil((async () => {
  for (const k of await caches.keys()) if (k !== CACHE) await caches.delete(k);
  await clients.claim();
})()));
addEventListener('fetch', (e) => {
  const url = new URL(e.request.url);
  if (e.request.method !== 'GET' || url.origin !== location.origin || !SHELL.includes(url.pathname)) return;
  e.respondWith((async () => {
    const cache = await caches.open(CACHE);
    try {
      const r = await fetch(e.request, { signal: AbortSignal.timeout(4000) });
      if (r.ok) await cache.put(url.pathname, r.clone());
      return r;
    } catch (err) {
      return (await cache.match(url.pathname)) || Response.error();
    }
  })());
});
