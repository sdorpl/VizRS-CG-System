// Service worker Trio na telefonie: uruchamia aplikację bez sieci, żeby operator zobaczył stan połączenia zamiast błędu przeglądarki.
// Zawsze najpierw sieć: po aktualizacji serwera telefon nie może grać starym interfejsem. Komend i stanu anteny nigdy nie zapisuje.
const CACHE = 'vizrs-mobile-1';
const SHELL = ['/mobile', '/ui.css', '/text-model.js', '/renderer.js', '/qr.js', '/ticker-model.js', '/richtext.js', '/motion.js', '/field-rules.js', '/mobile-icon.svg', '/mobile.webmanifest'];

addEventListener('install', () => skipWaiting());
addEventListener('activate', (e) => e.waitUntil(clients.claim()));
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
