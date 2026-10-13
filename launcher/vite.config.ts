import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { defineConfig, type Plugin } from "vite";

type ConfigLauncher = { nom: string; devise?: string; clePubliqueMaj?: string; couleurs?: Record<string, string> };

const CHEMIN = fileURLToPath(new URL("../launcher.config.json", import.meta.url));
const config: ConfigLauncher = JSON.parse(readFileSync(CHEMIN, "utf8"));

const echapper = (t: string) => t.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
const versAscii = (t: string) => t.replace(/[^\x20-\x7e]/g, (c) => `&#${c.codePointAt(0)};`);

function couleurs(): string {
  const lignes = Object.entries(config.couleurs ?? {})
    .filter(([cle, valeur]) => /^[a-z][a-z0-9-]*$/.test(cle) && /^[#a-zA-Z0-9(),.%\s/-]+$/.test(valeur))
    .map(([cle, valeur]) => `--${cle}: ${valeur};`);
  return lignes.length ? `<style>:root:root { ${lignes.join(" ")} }</style>` : "";
}

function configurerPage(): Plugin {
  return {
    name: "configurer-page",
    transformIndexHtml(html) {
      return html
        .replaceAll("%NOM_LAUNCHER%", versAscii(echapper(config.nom)))
        .replaceAll("%DEVISE%", versAscii(echapper(config.devise ?? "")))
        .replace("</head>", `${couleurs()}\n  </head>`);
    },
  };
}

export default defineConfig({
  clearScreen: false,
  server: { port: 1420, strictPort: true, watch: { ignored: ["**/src-tauri/**"] } },
  build: { target: "es2021", assetsInlineLimit: 0 },
  define: {
    __NOM_LAUNCHER__: JSON.stringify(config.nom),
    __MAJ_AUTO__: JSON.stringify((config.clePubliqueMaj ?? "").trim().length > 0),
  },
  plugins: [configurerPage()],
});
