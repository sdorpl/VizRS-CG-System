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
    ok(near(end.opacity, 1) && near(end.dx, 0) && near(end.dy, 0) && near(end.scale, 1) && (!end.clip || end.clip.every((v) => near(v, 0))) && !end.hidden && !end.scramble, `tekst IN ${type}: koniec neutralny`, end);
    ok(start.opacity < 0.01 || (start.clip && start.clip.some((v) => v > 0.99)) || start.hidden || start.scramble || start.count === 0, `tekst IN ${type}: początek ukryty`, start);
  }
  for (const type of MO.TEXT_OUT.filter((x) => x !== 'none')) {
    const s = { type, dur: 0.5 };
    const st = MO.textOutFx(s, 0, 5, 0), end = MO.textOutFx(s, 4, 5, 9);
    ok(near(st.opacity, 1) && near(st.dy, 0) && (!st.clip || st.clip.every((v) => near(v, 0))), `tekst OUT ${type}: początek neutralny`, st);
    ok(end.opacity < 0.01 || (end.clip && end.clip.some((v) => v > 0.99)), `tekst OUT ${type}: koniec ukryty`, end);
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
  ok(MO.textInFx({ type: 'clip-up', dur: 1, ease: 'linear' }, 0, 1, 0.25).clip.join() === '0.75,0,0,0', 'clip-up: przycięcie od góry jak w cgmotion (bez ruchu)');
  ok(MO.textOutFx({ type: 'hide', dur: 1, ease: 'linear' }, 0, 1, 0.5).clip.join() === '0,0,0,0.5', 'hide: przycięcie od lewej');
  ok(near(MO.textInFx({ type: 'blur', dur: 1, ease: 'linear' }, 0, 1, 0).blur, 20) && near(MO.textInFx({ type: 'words-blur', dur: 1, ease: 'linear' }, 0, 3, 0).blur, 14), 'rozmycie 20 px (całość) i 14 px (jednostki)');
  const scr = { type: 'scramble', dur: 1, ease: 'linear' };
  ok(!MO.textInFx(scr, 3, 10, 0.5).scramble && MO.textInFx(scr, 5, 10, 0.5).scramble, 'szyfrowanie: znaki gotowe wg postępu całego tekstu');
  ok(MO.scrambleChar(3, 1.0) === MO.scrambleChar(3, 1.01), 'szyfrowanie deterministyczne w kadrze');

  // pętle
  for (const type of MO.LOOP_TYPES) { const s = MO.loopState({ type }, 0.37); ok(Object.values(s).every((v) => v === null || Array.isArray(v) || Number.isFinite(v)), `pętla ${type}: wartości skończone`, s); }
  ok(MO.loopState({ type: 'pulse' }, 0.8).sx > 1.03, 'pulse: skala rośnie do ~1.04');
  ok(near(MO.loopState({ type: 'pulse', amp: 10 }, 0.8).sx, 1.1), 'pulse: siła = % skali');
  ok(MO.loopState({ type: 'shine', dur: 1 }, 0.5).shine > 0 && MO.loopState({ type: 'shine', dur: 1 }, 2).shine === -1, 'shine: przejazd i przerwa');

  // tryb schodkowy (lekkie /output?fx=step): kilka stanów na okres, ruch wyłączony
  const vals = (f) => new Set(Array.from({ length: 300 }, (_, i) => JSON.stringify(f(i / 100)))).size;
  ok(vals((t) => MO.loopStateStep({ type: 'pulse' }, t)) <= 4 && MO.loopStateStep({ type: 'pulse' }, 0.85).sx > 1.03, 'schodki: pulse – najwyżej 4 stany, szczyt w połowie okresu');
  ok(vals((t) => MO.loopStateStep({ type: 'breathe', dur: 2 }, t)) <= 4 && MO.loopStateStep({ type: 'breathe', dur: 2 }, 2.1).opacity < 0.7, 'schodki: breathe z własnym okresem');
  ok(['float', 'bounce', 'wobble', 'shake', 'shine', 'underline-sweep'].every((type) => vals((t) => MO.loopStateStep({ type }, t)) === 1 && MO.loopStateStep({ type }, 0.5).shine === -1), 'schodki: pętle ruchowe, połysk i podkreślenie wyłączone');
  ok(vals((t) => MO.bgStateStep('pulse', t, 1)) <= 4 && vals((t) => MO.bgStateStep('glow', t)) <= 4, 'schodki: tło pulse i glow – najwyżej 4 stany');
  ok(MO.bgStateStep('shimmer', 1).shimmer == null && MO.bgStateStep('scanline', 1).scan == null, 'schodki: shimmer i scanline wyłączone');

  // presety → klucze
  const rest = { x: 100, y: 50, opacity: 1, scale: 1, skewX: 0, blur: 0, clipL: 0, clipT: 0, clipR: 0, clipB: 0 };
  const k1 = MO.motionToKeys({ in: { type: 'slide', dir: 'left', dur: 0.5, ease: 'power2.out' }, out: { type: 'fade', dur: 0.4, delay: 0.1, ease: 'power2.in' } }, { rest, outStart: 2 });
  ok(k1.anim.x.length === 2 && k1.anim.x[0].v === 100 - 1920 && k1.anim.x[0].e === 'out' && k1.anim.x[1].t === 0.5 && k1.anim.x[1].v === 100, 'na klucze: wjazd = dwa klucze z krzywą out', k1.anim.x);
  ok(k1.anim.opacity.map((k) => [k.t, k.v]).join() === [[0, 0], [0.5, 1], [2.1, 1], [2.5, 0]].join(), 'na klucze: krycie IN i OUT na jednej ścieżce', k1.anim.opacity);
  const k2 = MO.motionToKeys({ in: { type: 'pop', dur: 0.4 } }, { rest, outStart: Infinity });
  ok(k2.anim.scale.length > 5 && k2.anim.scale.some((k) => k.v > 1) && k2.anim.scale.at(-1).v === 1, 'na klucze: wyskok próbkowany (przeskok > 1)', k2.anim.scale.length);
  const k3 = MO.motionToKeys({ in: { type: 'flip', dur: 0.4 }, out: { type: 'fade' } }, { rest, outStart: Infinity });
  ok(k3.done.length === 0 && k3.skipped.length === 2, 'na klucze: odwrócenie 3D i wyjście bez stopu pominięte', k3.skipped);
  const k4 = MO.motionToKeys({ in: { type: 'clip', dir: 'left', dur: 0.3, ease: 'linear' } }, { rest, outStart: 1, shift: 0.5 });
  ok(k4.anim.clipR?.[0].t === 0.5 && k4.anim.clipR[0].v === 100 && k4.anim.clipR[1].v === 0 && !k4.anim.x, 'na klucze: przycięcie z przesunięciem sekwencji', k4.anim);

  // klucze → presety (odwrotność): powrót z motionToKeys daje ten sam preset
  const lin = (anim) => (p, t) => { const tr = anim[p]; if (t <= tr[0].t) return tr[0].v; for (let i = 0; i < tr.length - 1; i++) { const a = tr[i], b = tr[i + 1]; if (t <= b.t) { const u = (t - a.t) / (b.t - a.t || 1); const E = { out: (x) => 1 - (1 - x) ** 3, in: (x) => x ** 3, linear: (x) => x }[a.e] || ((x) => x); return a.v + (b.v - a.v) * E(u); } } return tr.at(-1).v; };
  const m1 = MO.keysToMotion(k1.anim, { props: rest, sample: lin(k1.anim), outStart: 2 });
  ok(m1.done.join() === 'in,out' && m1.motion.in.type === 'slide' && m1.motion.in.dir === 'left' && m1.motion.in.dur === 0.5 && m1.motion.in.ease === 'power2.out', 'z kluczy: wjazd z lewej', m1.motion);
  ok(m1.motion.out.type === 'fade' && m1.motion.out.delay === 0.1 && m1.motion.out.dur === 0.4 && m1.motion.out.ease === 'power2.in' && !Object.keys(m1.anim).length && m1.props.x === 100, 'z kluczy: wyjście przenikaniem, klucze usunięte, spoczynek w props', m1);
  const a5 = { y: [{ t: 0.2, v: 900, e: 'out' }, { t: 0.7, v: 700 }] };
  const m5 = MO.keysToMotion(a5, { props: rest, sample: lin(a5), outStart: Infinity });
  ok(!m5.done.length && m5.skipped.length === 1, 'z kluczy: krótki wjazd 200 px nie jest presetem', m5.skipped);
  const m6 = MO.keysToMotion(a5, { props: rest, sample: lin(a5), outStart: Infinity, approx: true, shift: 0.5 });
  ok(m6.done[0] === 'in' && m6.motion.in.type && m6.approx.length === 1 && m6.dropAfter && m6.motion.in.delay === 0.2 && m6.props.y === 700, 'z kluczy: przybliżenie na prośbę, bez sekwencji', m6);

  console.log(failures ? `\n${failures} FAIL / ${checks}` : `Ruch zgodny (${checks} sprawdzeń)`);
  process.exit(failures ? 1 : 0);
})();
