// Conduce la interfaz real de IureDav (Vite + backend simulado de mock.ts) y saca
// las capturas del README y los fotogramas del GIF.
//
//   node scripts/readme-media/capture.mjs [puerto]       (por defecto 5303)
//
// Salida: scripts/readme-media/out/<tema>-<idioma>.png  (capturas a 1.5x)
//         scripts/readme-media/out/gif-<idioma>/NNN.png + frames.txt (concat de ffmpeg)
import puppeteer from 'puppeteer-core';
import fs from 'node:fs';

const PORT = process.argv[2] || '5303';
const BASE = `http://localhost:${PORT}/scripts/readme-media/app.html`;
const OUT = new URL('./out/', import.meta.url).pathname;
const CHROME = process.env.CHROME || process.env.HOME + '/.cache/ms-playwright/chromium-1234/chrome-linux64/chrome';
fs.rmSync(OUT, { recursive: true, force: true });
fs.mkdirSync(OUT, { recursive: true });

const browser = await puppeteer.launch({ executablePath: CHROME, args: ['--no-sandbox', '--font-render-hinting=none'] });
const wait = (ms) => new Promise((r) => setTimeout(r, ms));

const QUIETO = `*{caret-color:transparent!important;transition:none!important;animation:none!important}
html{scroll-behavior:auto!important} ::-webkit-scrollbar{display:none}`;

async function abrir({ lang, params = '', width = 1280, height = 800, dark = false }) {
  const p = await browser.newPage();
  p.on('pageerror', (e) => console.log('ERR:', e.message));
  p.on('console', (m) => { if (m.text().includes('mock:')) console.log(m.text()); });
  await p.emulateMediaFeatures([{ name: 'prefers-color-scheme', value: dark ? 'dark' : 'light' }]);
  await p.setViewport({ width, height, deviceScaleFactor: 1.5 });
  await p.goto(`${BASE}?lang=${lang}${params}`, { waitUntil: 'networkidle0' });
  await p.addStyleTag({ content: QUIETO });
  await wait(500);
  return p;
}

/** Pulsa el botón (o la etiqueta) cuyo texto empieza por `texto`; `n` para el enésimo. */
async function pulsar(p, texto, n = 0) {
  const ok = await p.evaluate((texto, n) => {
    const els = [...document.querySelectorAll('button, label')].filter((b) => b.textContent.trim().startsWith(texto));
    if (!els[n]) return false;
    els[n].click();
    return true;
  }, texto, n);
  if (!ok) throw new Error(`no encuentro «${texto}»`);
  await wait(250);
}
const scrollA = (p, y) => p.evaluate((y) => window.scrollTo(0, y), y);
const topDe = (p, texto) => p.evaluate((texto) => {
  const el = [...document.querySelectorAll('h2, h3, strong, button')].find((e) => e.textContent.trim().startsWith(texto));
  return el ? el.getBoundingClientRect().top + window.scrollY : 0;
}, texto);
const resolver = (p, clave) => p.evaluate((c) => window.__resolver(c), clave);

const T = {
  en: { ver: 'See what this server can do', subidas: 'Also check whether it accepts uploads', volver: 'Back', montar: 'Mount',
        anadir: 'Add connection', conectar: 'Connect and get access', discrepa: 'This server advertises', comprobado: 'What was checked', offline: 'Available offline' },
  es: { ver: 'Ver qué sabe hacer este servidor', subidas: 'Comprobar también si acepta subidas', volver: 'Volver', montar: 'Montar',
        anadir: 'Añadir conexión', conectar: 'Conectar y obtener acceso', discrepa: 'Este servidor anuncia', comprobado: 'Lo que se comprobó', offline: 'Disponible sin conexión' },
};

for (const lang of ['en', 'es']) {
  const t = T[lang];

  // 1. Ventana principal con la unidad de Iurefficient montada.
  {
    const p = await abrir({ lang, params: '&montado=1&medido=1' });
    await p.screenshot({ path: `${OUT}main-${lang}.png` });
    await p.close();
  }

  // 2. Resultado de la sonda: anunciado contra real.
  {
    const p = await abrir({ lang, params: '&medido=1' });
    await pulsar(p, t.ver);
    await scrollA(p, (await topDe(p, t.comprobado)) - 250);
    await wait(150);
    await p.screenshot({ path: `${OUT}probe-${lang}.png` });
    await p.close();
  }

  // 3. Carpetas sin conexión (tema oscuro), una descargándose.
  {
    const p = await abrir({ lang, params: '&montado=1&medido=1', dark: true });
    await p.evaluate(() => {
      const base = '/home/demo/Iurefficient/';
      window.__emit('iuredav://anclaje', { conexion: 'iurefficient', carpeta: base + 'Casos/Ruiz & Asociados — Arrendamiento', archivos: 37, bytes: 51200000, fallidos: 0, terminado: false });
      window.__emit('iuredav://anclaje', { conexion: 'iurefficient', carpeta: base + 'General/Plantillas', archivos: 128, bytes: 9400000, fallidos: 0, terminado: true });
    });
    await wait(300);
    await scrollA(p, (await topDe(p, 'Iurefficient')) - 40);
    await wait(150);
    await p.screenshot({ path: `${OUT}offline-${lang}.png` });
    await p.close();
  }

  // 4. Conexión nueva con la cuenta de Iurefficient (sin contraseña de aplicación a mano).
  {
    const p = await abrir({ lang });
    await pulsar(p, t.anadir);
    await p.type('#passCuenta', 'demo-password');
    await pulsar(p, t.conectar);
    await wait(300);
    await p.evaluate(() => document.activeElement?.blur());
    await p.evaluate(() => {
      const a = document.querySelector('.tarjeta .acciones').getBoundingClientRect();
      window.scrollTo(0, a.bottom + window.scrollY + 40 - window.innerHeight);
    });
    await p.screenshot({ path: `${OUT}add-${lang}.png` });
    await p.close();
  }

  // GIF: la sonda mide cada operación, deduce cómo montar y la unidad queda montada.
  {
    const dir = `${OUT}gif-${lang}/`;
    fs.mkdirSync(dir, { recursive: true });
    const lista = [];
    let n = 0;
    const p = await abrir({ lang, width: 1000, height: 680 });
    const foto = async (seg) => {
      const f = `${String(n++).padStart(3, '0')}.png`;
      await p.screenshot({ path: dir + f });
      lista.push(`file '${f}'\nduration ${seg}`);
    };
    const desplazar = async (desde, hasta, pasos, seg) => {
      for (let i = 1; i <= pasos; i++) {
        const k = i / pasos;
        const e = k < 0.5 ? 2 * k * k : 1 - Math.pow(-2 * k + 2, 2) / 2;
        await scrollA(p, Math.round(desde + (hasta - desde) * e));
        await foto(seg);
      }
    };

    await pulsar(p, t.ver);
    await foto(1.1);
    await pulsar(p, t.subidas);           // el confirm() se acepta solo en el mock
    await foto(0.9);                       // «Comprobando…»
    await resolver(p, 'resondear');
    await wait(300);
    await foto(1.0);
    const fin = await p.evaluate(() => document.documentElement.scrollHeight - window.innerHeight);
    const tabla = Math.min(fin, (await topDe(p, t.comprobado)) - 20);
    await desplazar(0, tabla, 10, 1 / 12);
    await foto(1.6);
    await scrollA(p, 0);
    await pulsar(p, t.volver);
    await scrollA(p, (await topDe(p, 'Iurefficient')) - 200);
    await foto(0.9);
    await pulsar(p, t.montar);
    await foto(0.5);                       // «…»
    await resolver(p, 'montar');
    await wait(300);
    await foto(2.0);
    lista.push(`file '${String(n - 1).padStart(3, '0')}.png'`); // el concat de ffmpeg ignora la última duración
    fs.writeFileSync(dir + 'frames.txt', lista.join('\n') + '\n');
    await p.close();
  }
}

await browser.close();
console.log('listo:', OUT);
