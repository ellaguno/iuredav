#!/usr/bin/env python3
"""Doble de pruebas del WebDAV de Iurefficient.

Imita el comportamiento medido contra INST-002, incluida la parte incomoda: la
cabecera `Allow:` promete DELETE, COPY, MOVE, MKCOL y PROPPATCH, pero al usarlos
el servidor los rechaza. Sirve para comprobar que la sonda detecta la mentira sin
tocar una instancia real ni los documentos de ningun cliente.

    python3 tests/servidor-falso.py [puerto]

Usuario: prueba@ejemplo.com   Contrasena: iurdav_falso
"""
import base64
import sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

USUARIO, PASSWORD = "prueba@ejemplo.com", "iurdav_falso"
RAIZ = "/webdav/"

# El arbol virtual: colecciones y documentos con su contenido.
COLECCIONES = {"", "Casos/", "Casos/Expediente 1/", "General/"}
DOCUMENTOS = {
    "General/Análisis de Peña.docx": b"contenido de ejemplo con nombre acentuado",
    "Casos/Expediente 1/demanda.pdf": b"%PDF-1.4 falso",
}

# Lo que el servidor ANUNCIA. Casi todo es mentira, y de eso va la prueba.
ALLOW = "OPTIONS, GET, HEAD, PROPFIND, PUT, DELETE, COPY, MOVE, MKCOL, PROPPATCH, LOCK, UNLOCK"


def xml_escape(s):
    return s.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")


def quote(s):
    return "".join(c if c.isalnum() or c in "/-_.~" else "".join(f"%{b:02X}" for b in c.encode()) for c in s)


class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, fmt, *a):
        sys.stderr.write("  [servidor-falso] %s %s\n" % (self.command, self.path))

    # ---------------------------------------------------------------- helpers
    def autorizado(self):
        h = self.headers.get("Authorization", "")
        if not h.startswith("Basic "):
            return False
        try:
            u, _, p = base64.b64decode(h[6:]).decode().partition(":")
        except Exception:
            return False
        return u == USUARIO and p == PASSWORD

    def responder(self, codigo, cuerpo=b"", cabeceras=None):
        if isinstance(cuerpo, str):
            cuerpo = cuerpo.encode()
        self.send_response(codigo)
        for k, v in (cabeceras or {}).items():
            self.send_header(k, v)
        self.send_header("Content-Length", str(len(cuerpo)))
        self.end_headers()
        if self.command != "HEAD":
            self.wfile.write(cuerpo)

    def rel(self):
        p = self.path
        return p[len(RAIZ):] if p.startswith(RAIZ) else None

    def guardia(self):
        """Devuelve la ruta relativa, o None si ya se respondio con un error."""
        if not self.autorizado():
            self.responder(401, "no autorizado", {"WWW-Authenticate": 'Basic realm="iurefficient"'})
            return None
        r = self.rel()
        if r is None:
            self.responder(404, "fuera de /webdav/")
            return None
        return r

    # ---------------------------------------------------------------- verbos
    def do_OPTIONS(self):
        # Anuncia de todo. Ahi esta el problema que motiva la sonda.
        self.responder(200, b"", {"Allow": ALLOW, "DAV": "1, 2", "Server": "WsgiDAV/4.3.3 (falso)"})

    def do_PROPFIND(self):
        if (rel := self.guardia()) is None:
            return
        self._leer_cuerpo()
        if rel not in COLECCIONES and rel not in DOCUMENTOS:
            return self.responder(404, "no existe")

        depth = self.headers.get("Depth", "1")
        hijos = [rel] if rel in COLECCIONES else []
        if rel in COLECCIONES and depth != "0":
            prefijo = rel
            for c in COLECCIONES:
                if c and c != rel and c.startswith(prefijo) and c[len(prefijo):].count("/") == 1:
                    hijos.append(c)
            for d in DOCUMENTOS:
                if d.startswith(prefijo) and "/" not in d[len(prefijo):]:
                    hijos.append(d)
        elif rel in DOCUMENTOS:
            hijos = [rel]

        partes = ['<?xml version="1.0" encoding="utf-8" ?>', '<D:multistatus xmlns:D="DAV:">']
        for h in hijos:
            href = xml_escape(quote(RAIZ + h))
            if h in COLECCIONES or h.endswith("/"):
                props = "<D:resourcetype><D:collection/></D:resourcetype>"
            else:
                # Sin getetag: este servidor no ofrece hashes ni etags, y eso es
                # justo lo que impide detectar cambios por contenido.
                props = (
                    "<D:resourcetype/>"
                    f"<D:getcontentlength>{len(DOCUMENTOS[h])}</D:getcontentlength>"
                    "<D:getlastmodified>Tue, 09 Sep 2026 12:00:00 GMT</D:getlastmodified>"
                )
            partes.append(
                f"<D:response><D:href>{href}</D:href><D:propstat><D:prop>{props}</D:prop>"
                "<D:status>HTTP/1.1 200 OK</D:status></D:propstat></D:response>"
            )
        partes.append("</D:multistatus>")
        self.responder(207, "\n".join(partes), {"Content-Type": "application/xml; charset=utf-8"})

    def do_GET(self):
        if (rel := self.guardia()) is None:
            return
        from urllib.parse import unquote
        rel = unquote(rel)
        if rel not in DOCUMENTOS:
            return self.responder(404, "no existe")
        datos = DOCUMENTOS[rel]

        rango = self.headers.get("Range")
        if rango and rango.startswith("bytes="):
            ini, _, fin = rango[6:].partition("-")
            i = int(ini or 0)
            f = min(int(fin), len(datos) - 1) if fin else len(datos) - 1
            trozo = datos[i:f + 1]
            return self.responder(206, trozo, {
                "Content-Range": f"bytes {i}-{f}/{len(datos)}",
                "Accept-Ranges": "bytes",
            })
        self.responder(200, datos, {"Accept-Ranges": "bytes"})

    do_HEAD = do_GET

    def do_PUT(self):
        if (rel := self.guardia()) is None:
            return
        datos = self._leer_cuerpo()
        nuevo = rel not in DOCUMENTOS
        # Un PUT sobre un documento existente no sobrescribe: crea una version.
        # Desde fuera solo se aprecia en que el GET devuelve lo nuevo.
        DOCUMENTOS[rel] = datos
        self.responder(201 if nuevo else 204)

    def do_DELETE(self):
        if self.guardia() is None:
            return
        self.responder(403, "los documentos se eliminan desde Iurefficient")

    def do_MKCOL(self):
        if self.guardia() is None:
            return
        self.responder(403, "las carpetas se crean desde Iurefficient")

    def do_MOVE(self):
        if self.guardia() is None:
            return
        # No es un rechazo limpio: el proveedor revienta. Para un cliente es peor,
        # porque no se distingue de una caida temporal.
        self.responder(502, "puerta de enlace incorrecta")

    do_COPY = do_MOVE

    def do_PROPPATCH(self):
        if self.guardia() is None:
            return
        self._leer_cuerpo()
        # 207 por fuera, 403 por dentro: si el cliente solo mira el codigo exterior,
        # cree que fijo la fecha cuando en realidad no la fijo.
        cuerpo = (
            '<?xml version="1.0" encoding="utf-8" ?>'
            '<D:multistatus xmlns:D="DAV:"><D:response>'
            f"<D:href>{xml_escape(quote(self.path))}</D:href>"
            "<D:propstat><D:prop><D:getlastmodified/></D:prop>"
            "<D:status>HTTP/1.1 403 Forbidden</D:status></D:propstat>"
            "</D:response></D:multistatus>"
        )
        self.responder(207, cuerpo, {"Content-Type": "application/xml; charset=utf-8"})

    def do_LOCK(self):
        if self.guardia() is None:
            return
        self._leer_cuerpo()
        # Concede SIEMPRE, incluso sobre algo ya bloqueado: los locks viven en la
        # memoria de cada worker de gunicorn, asi que ninguno ve los de los demas.
        cuerpo = (
            '<?xml version="1.0" encoding="utf-8" ?>'
            '<D:prop xmlns:D="DAV:"><D:lockdiscovery><D:activelock>'
            "<D:locktype><D:write/></D:locktype><D:lockscope><D:exclusive/></D:lockscope>"
            "<D:locktoken><D:href>opaquelocktoken:falso-1234</D:href></D:locktoken>"
            "</D:activelock></D:lockdiscovery></D:prop>"
        )
        self.responder(200, cuerpo, {
            "Lock-Token": "<opaquelocktoken:falso-1234>",
            "Content-Type": "application/xml; charset=utf-8",
        })

    def do_UNLOCK(self):
        if self.guardia() is None:
            return
        self.responder(204)

    def _leer_cuerpo(self):
        n = int(self.headers.get("Content-Length") or 0)
        return self.rfile.read(n) if n else b""


if __name__ == "__main__":
    puerto = int(sys.argv[1]) if len(sys.argv) > 1 else 8099
    print(f"servidor falso en http://127.0.0.1:{puerto}{RAIZ}", file=sys.stderr)
    print(f"  usuario: {USUARIO}   contrasena: {PASSWORD}", file=sys.stderr)
    ThreadingHTTPServer(("127.0.0.1", puerto), Handler).serve_forever()
