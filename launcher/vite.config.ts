import { existsSync, readFileSync } from "node:fs";
import { extname, isAbsolute, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig, type Plugin } from "vite";

type ConfigLauncher = { nom: string; devise?: string; clePubliqueMaj?: string; imageFond?: string; couleurs?: Record<string, string> };

const CHEMIN = fileURLToPath(new URL("../launcher.config.json", import.meta.url));
const DEPOT = fileURLToPath(new URL("..", import.meta.url));
const PUBLIC = fileURLToPath(new URL("./public", import.meta.url));
const config: ConfigLauncher = JSON.parse(readFileSync(CHEMIN, "utf8"));

const echapper = (t: string) => t.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
const versAscii = (t: string) => t.replace(/[^\x20-\x7e]/g, (c) => `&#${c.codePointAt(0)};`);

const TYPES_IMAGE: Record<string, string> = { ".webp": "image/webp", ".jpg": "image/jpeg", ".jpeg": "image/jpeg", ".png": "image/png", ".avif": "image/avif" };

type Fond = { url: string; source: string | null };

// "imageFond" : chemin d'un fichier de launcher/public/ (servi tel quel) ou chemin relatif a la racine du depot
// (copie dans la construction sous /fond-perso.<ext>). Vide ou absent : pas d'image, degrade seul.
function imageFond(): Fond | null {
  const nom = (config.imageFond ?? "").trim();
  if (!nom) return null;
  const ext = extname(nom).toLowerCase();
  if (!TYPES_IMAGE[ext]) throw new Error(`launcher.config.json : "imageFond" doit etre une image ${Object.keys(TYPES_IMAGE).join(", ")}`);
  const dansPublic = resolve(PUBLIC, nom);
  if (!isAbsolute(nom) && dansPublic.startsWith(PUBLIC + sep) && existsSync(dansPublic)) {
    return { url: "/" + relative(PUBLIC, dansPublic).split(sep).map(encodeURIComponent).join("/"), source: null };
  }
  const dansDepot = resolve(DEPOT, nom);
  if (existsSync(dansDepot)) return { url: `/fond-perso${ext}`, source: dansDepot };
  throw new Error(`launcher.config.json : "imageFond" introuvable (${nom}), ni dans launcher/public/ ni depuis la racine du depot`);
}

const fond = imageFond();

function variables(): string {
  const lignes = Object.entries(config.couleurs ?? {})
    .filter(([cle, valeur]) => /^[a-z][a-z0-9-]*$/.test(cle) && /^[#a-zA-Z0-9(),.%\s/-]+$/.test(valeur))
    .map(([cle, valeur]) => `--${cle}: ${valeur};`);
  if (fond) lignes.push(`--image-fond: url("${fond.url}");`);
  return lignes.length ? `<style>:root:root { ${lignes.join(" ")} }</style>` : "";
}

function configurerPage(): Plugin {
  return {
    name: "configurer-page",
    transformIndexHtml(html) {
      return html
        .replaceAll("%NOM_LAUNCHER%", versAscii(echapper(config.nom)))
        .replaceAll("%DEVISE%", versAscii(echapper(config.devise ?? "")))
        .replace("</head>", `${variables()}\n  </head>`);
    },
    configureServer(serveur) {
      if (!fond?.source) return;
      const source = fond.source;
      serveur.middlewares.use(fond.url, (_req, res) => {
        res.setHeader("Content-Type", TYPES_IMAGE[extname(source).toLowerCase()]);
        res.end(readFileSync(source));
      });
    },
    generateBundle() {
      if (fond?.source) this.emitFile({ type: "asset", fileName: fond.url.slice(1), source: readFileSync(fond.source) });
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
