import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";

try {
  const saved = localStorage.getItem("mimi-appearance");
  const dark = saved === "dark" || (saved === "system" && window.matchMedia("(prefers-color-scheme: dark)").matches);
  document.documentElement.dataset.theme = dark ? "dark" : "light";
} catch {
  // Without a saved choice, the light colors show until the settings load.
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
