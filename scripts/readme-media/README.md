# Capturas y GIF del README

Salen de la interfaz real (`src/`, servida por Vite) con el backend de Tauri
simulado en `mock.ts` (`@tauri-apps/api/mocks`). Solo datos ficticios:
`demo.iurefficient.com`, `dav.example.com`, `/home/demo`, «Ana Torres».

```bash
# desde la raíz del repo
npm i --no-save puppeteer-core                 # no se añade a package.json
npx vite --port 5303 --strictPort &            # sirve scripts/readme-media/app.html
node scripts/readme-media/capture.mjs 5303     # capturas y fotogramas -> scripts/readme-media/out/
scripts/readme-media/build.sh                  # -> docs/media/*.png y hero-{en,es}.gif
kill %1
```

Requiere Chromium (por defecto `~/.cache/ms-playwright/chromium-1234/chrome-linux64/chrome`,
o la variable `CHROME`), ImageMagick, `pngquant` y `ffmpeg`.

- `app.html`: carga `mock.ts` antes que `src/main.tsx`.
- `mock.ts`: conexiones, mediciones (las del doble `tests/servidor-falso.py`) y
  respuestas en inglés o español según `?lang=`.
- `capture.mjs`: conduce la interfaz y guarda las capturas (1280×800 a 1.5x) y los
  fotogramas del GIF con su duración (`frames.txt`, formato concat de ffmpeg).
- `build.sh`: reduce a 1600 px, comprime con pngquant y arma los GIF con paleta
  en dos pasadas.
