// Trio – punkt wejścia: inicjalizacja modułów w kolejności i zdarzenia z serwera.

import { init as coreInit } from './core.js';
import { init as pagesInit } from './pages.js';
import { init as previewInit } from './preview.js';
import { init as fieldsInit } from './fields.js';
import { init as onairInit } from './onair.js';
import { init as rundownInit } from './rundown.js';
import { init as systemInit } from './system.js';
import { init as mobileInit } from './mobile.js';
import { connectEvents } from '/renderer.js';
import { $, loadShow, loadState, phone, releaseOwnLocks, toast } from './core.js';
import { syncPv } from './pages.js';
import { setConnected } from './system.js';
import { applyLayout, keepAwake, syncPgm } from './mobile.js';

// ------------------------------------------------------------ zdarzenia z serwera
let closeEvents = null, lost = false;
function listen() {
  closeEvents?.();
  closeEvents = connectEvents((ev) => {
    if (ev.t === 'show') { loadShow().catch(() => {}); return; }
    loadState();
  }, (ok) => {
    setConnected(ok);
    $('#conn').textContent = ok ? 'połączono' : 'brak połączenia';
    $('#conn').className = 'pill ' + (ok ? 'ok' : 'on');
    // przerwa w strumieniu = mogliśmy przegapić zmiany, także w show
    if (ok && lost) loadShow().then(loadState).catch(() => {}); else if (ok) loadState();
    lost = !ok;
  });
}

coreInit();
pagesInit();
previewInit();
fieldsInit();
onairInit();
rundownInit();
systemInit();
mobileInit();
document.addEventListener('visibilitychange', () => {
  syncPgm(); syncPv();
  if (document.hidden || !phone()) return;
  // przeglądarka telefonu potrafi zamknąć strumień w tle bez zdarzenia błędu
  lost = true; listen(); keepAwake();
});
listen(); applyLayout(); keepAwake();
releaseOwnLocks().then(loadShow).then(loadState).catch((e) => toast(e.message, true));
