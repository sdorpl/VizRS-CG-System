// Testy modułu ruchu (web/motion.js – presety IN/OUT, efekty tekstu, pętle, sekwencja):  node tests/motion.js
const path = require('path');
const { pathToFileURL } = require('url');

let failures = 0, checks = 0;
const ok = (cond, msg, info) => { checks++; if (!cond) { failures++; console.log('FAIL ' + msg + (info !== undefined ? ' → ' + JSON.stringify(info) : '')); } };
const near = (a, b, e = 1e-3) => Math.abs(a - b) < e;

(async () => {
  const MO = await import(pathToFileURL(path.join(__dirname, '../web/motion.js')).href);

  // krzywe: końce 0 i 1, aliasy, wartości wzorcowe GSAP
  for (const e of [...MO.MOTION_EASES, 'power1.out', 'sine.in', 'expo.in', 'back.inOut', 'bounce.in', 'elastic.in', 'cubic-bezier(.4,0,.2,1)', 'nieznana']) {
    ok(near(MO.ease(e, 0), 0) && near(MO.ease(e, 1), 1), `krzywa ${e}: 0→0, 1→1`, [MO.ease(e, 0), MO.ease(e, 1)]);
  }
  ok(near(MO.ease('power2.out', 0.5), 0.875), 'power2.out(0.5) = 0.875 (cubic)');
  ok(near(MO.ease('linear', 0.3), 0.3), 'linear');
  ok(MO.ease('back.out(1.7)', 0.6) > 1, 'back.out przeskakuje ponad 1');
  ok(near(MO.ease('spring', 0.5), MO.ease('back.out(1.4)', 0.5)), 'alias spring = back.out(1.4)');
  ok(near(MO.ease('power2.out', 2), 1) && near(MO.ease('power2.out', -1), 0), 'krzywa przycina postęp do [0, 1]');

  // presety: IN na końcu i OUT na początku = stan neutralny (glitch OUT skacze od razu – jak w BTB); IN na początku ukrywa
  const neutral = (s) => near(s.tx, 0) && near(s.ty, 0) && near(s.sx, 1) && near(s.sy, 1) && near(s.rot, 0) && near(s.skew, 0) && near(s.opacity, 1) && near(s.blur, 0)
    && near(s.flipX, 0) && near(s.flipY, 0) && near(s.shutter, 1) && near(s.rgb, 0) && (!s.clip || (near(s.clip[0], 0) && near(s.clip[1], 0) && near(s.clip[2], 1) && near(s.clip[3], 1))) && (s.diag < 0 || near(s.diag, 1));
  for (const type of MO.ANIM_TYPES) {
    for (const dir of MO.DIRECTIONS) {
      ok(neutral(MO.stateIn(type, dir, 'power2.out', 1)), `IN ${type}/${dir}: koniec = stan neutralny`, MO.stateIn(type, dir, 'power2.out', 1));
      if (type !== 'none' && type !== 'glitch') ok(neutral(MO.stateOut(type, dir, 'power2.in', 0)), `OUT ${type}/${dir}: początek = stan neutralny`, MO.stateOut(type, dir, 'power2.in', 0));
    }
  }
  const hidden = (s) => s.opacity < 0.01 || near(s.shutter, 0) || near(s.diag, 0) || (s.clip && (s.clip[2] - s.clip[0] < 0.01 || s.clip[3] - s.clip[1] < 0.01)) || Math.abs(s.tx) >= 1000 || Math.abs(s.ty) >= 1000;
  for (const type of MO.ANIM_TYPES.filter((x) => x !== 'none' && x !== 'bar-grow')) {
    ok(hidden(MO.stateIn(type, 'left', 'power2.out', 0)), `IN ${type}: początek ukryty`, MO.stateIn(type, 'left', 'power2.out', 0));
    ok(hidden(MO.stateOut(type, 'left', 'power2.in', 1)), `OUT ${type}: koniec ukryty`, MO.stateOut(type, 'left', 'power2.in', 1));
  }
  ok(MO.stateIn('slide', 'left', 'linear', 0).tx === -1920 && MO.stateIn('slide', 'bottom', 'linear', 0).ty === 1080, 'slide: z lewej -1920, z dołu +1080');
  ok(MO.stateIn('wipe', 'right', 'linear', 0.5).origin[0] === 1, 'wipe z prawej: punkt odniesienia po prawej');

  // czas: opóźnienie, OUT od ostatniego stopu, złożenie
  const m = { in: { type: 'fade', dur: 1, delay: 0.5, ease: 'linear' }, out: { type: 'fade', dur: 0.5, ease: 'linear' } };
  ok(near(MO.motionAt(m, 0.25, 3).opacity, 0), 'przed opóźnieniem ukryty');
  ok(near(MO.motionAt(m, 1, 3).opacity, 0.5), 'połowa wejścia');
  ok(near(MO.motionAt(m, 2, 3).opacity, 1), 'stoi');
  ok(near(MO.motionAt(m, 3.25, 3).opacity, 0.5), 'połowa wyjścia');
  ok(near(MO.motionAt(m, 2, Infinity).opacity, 1), 'bez stopu – brak wyjścia');
  ok(near(MO.inEnd(m), 1.5) && near(MO.outLen(m), 0.5), 'inEnd / outLen');
  ok(MO.holding({ ...m, loop: { type: 'pulse' } }, 2, 3) && !MO.holding(m, 1, 3) && !MO.holding(m, 3.1, 3), 'pętla tylko gdy grafika stoi');
  ok(near(MO.evalIn({ type: 'none', delay: 1 }, 0.5).opacity, 0) && near(MO.evalIn({ type: 'none', delay: 1 }, 1).opacity, 1), 'none: pojawia się po opóźnieniu');

  // sekwencja „po węźle”
  const nodes = [{ id: 'a', motion: { in: { type: 'fade', dur: 0.5 } } }, { id: 'b', motion: { after: 'a', in: { type: 'slide', dur: 0.4, delay: 0.1 } } },
    { id: 'g', children: [{ id: 'c', motion: { after: 'b', in: { type: 'fade', dur: 0.2 } } }] }, { id: 'x', motion: { after: 'y' } }, { id: 'y', motion: { after: 'x' } }];
  const sh = MO.sequenceShifts(nodes);
  ok(near(sh.get('a'), 0) && near(sh.get('b'), 0.5) && near(sh.get('c'), 1.0), 'po węźle: przesunięcia się sumują (także w grupie)', [...sh]);
  ok(Number.isFinite(sh.get('x')) && Number.isFinite(sh.get('y')), 'cykl w sekwencji nie zawiesza');
  ok(near(MO.motionAt(nodes[1].motion, 0.55, Infinity, sh.get('b')).tx, -1920), 'przesunięcie sekwencji opóźnia wejście');

  // efekty tekstu
  for (const type of MO.TEXT_IN.filter((x) => x !== 'none')) {
    const s = { type, dur: 0.5 };
    const end = MO.textInFx(s, 0, 5, 5), start = MO.textInFx(s, 0, 5, -0.01);
    ok(near(end.opacity, 1) && near(end.dx, 0) && near(end.dy, 0) && near(end.scale, 1) && near(end.revealX, 1) && near(end.clipY, 0) && !end.hidden && !end.scramble, `tekst IN ${type}: koniec neutralny`, end);
    ok(start.opacity < 0.01 || start.revealX < 0.01 || Math.abs(start.clipY) > 0.99 || start.hidden || start.scramble || start.count === 0, `tekst IN ${type}: początek ukryty`, start);
  }
  for (const type of MO.TEXT_OUT.filter((x) => x !== 'none')) {
    const s = { type, dur: 0.5 };
    const st = MO.textOutFx(s, 0, 5, 0), end = MO.textOutFx(s, 4, 5, 9);
    ok(near(st.opacity, 1) && near(st.dy, 0) && near(st.revealX, 1) && near(st.clipY, 0), `tekst OUT ${type}: początek neutralny`, st);
    ok(end.opacity < 0.01 || end.revealX < 0.01 || Math.abs(end.clipY) > 0.99, `tekst OUT ${type}: koniec ukryty`, end);
  }
  ok(MO.splitOf('chars-up') === 'chars' && MO.splitOf('words-fade') === 'words' && MO.splitOf('lines-clip') === 'lines' && MO.splitOf('fade') === 'whole' && MO.splitOf('scramble') === 'chars', 'podział jednostek');
  const st3 = { type: 'words-up', dur: 0.5, stagger: 0.1 };
  ok(MO.textInFx(st3, 0, 3, 0.1).opacity > MO.textInFx(st3, 2, 3, 0.1).opacity, 'odstęp: pierwsze słowo wyprzedza ostatnie');
  ok(MO.textInFx({ ...st3, from: 'end' }, 2, 3, 0.1).opacity > MO.textInFx({ ...st3, from: 'end' }, 0, 3, 0.1).opacity, 'kolejność od końca');
  ok(MO.counterText('Wynik: 1 250,5 zł', 0.5) === 'Wynik: 625,3 zł' || MO.counterText('Wynik: 1 250,5 zł', 0.5) === 'Wynik: 625,2 zł', 'licznik: dziesiętne z przecinkiem', MO.counterText('Wynik: 1 250,5 zł', 0.5));
  ok(MO.counterText('Frekwencja 12 500 osób', 1) === 'Frekwencja 12 500 osób', 'licznik: separator tysięcy zachowany');
  ok(MO.counterText('bez liczb', 0.3) === 'bez liczb', 'licznik: tekst bez liczb bez zmian');
  ok(MO.textUnits('Ala ma {b}kota', 'words') === 3 && MO.textUnits('Ala ma', 'chars') === 5 && MO.textUnits('a\nb\n', 'lines') === 2, 'liczba jednostek tekstu');
  ok(near(MO.inEnd({ textIn: { type: 'words-up', dur: 0.5 } }, 0, 'Ala ma kota'), 0.5 + 0.18) && near(MO.outLen({ textOut: { type: 'fade', dur: 0.4 } }, 'x'), 0.4), 'czas efektu tekstu z odstępami');
  ok(MO.scrambleChar(3, 1.0) === MO.scrambleChar(3, 1.01), 'szyfrowanie deterministyczne w kadrze');

  // pętle
  for (const type of MO.LOOP_TYPES) { const s = MO.loopState({ type }, 0.37); ok(Object.values(s).every((v) => v === null || Array.isArray(v) || Number.isFinite(v)), `pętla ${type}: wartości skończone`, s); }
  ok(MO.loopState({ type: 'pulse' }, 0.8).sx > 1.03, 'pulse: skala rośnie do ~1.04');
  ok(near(MO.loopState({ type: 'pulse', amp: 10 }, 0.8).sx, 1.1), 'pulse: siła = % skali');
  ok(MO.loopState({ type: 'shine', dur: 1 }, 0.5).shine > 0 && MO.loopState({ type: 'shine', dur: 1 }, 2).shine === -1, 'shine: przejazd i przerwa');

  console.log(failures ? `\n${failures} FAIL / ${checks}` : `Ruch zgodny (${checks} sprawdzeń)`);
  process.exit(failures ? 1 : 0);
})();
