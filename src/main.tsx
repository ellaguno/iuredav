import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./estilos.css";
import { cargarIdioma } from "./i18n";

// Primero el idioma, para no pintar la ventana en uno y repintarla en otro.
void cargarIdioma().then(() => {
  ReactDOM.createRoot(document.getElementById("root")!).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
  );
});
