// Applique launcher.config.json (racine du depot) aux fichiers qui ne savent pas le lire eux-memes :
// src-tauri/tauri.conf.json, package.json, package-lock.json, src-tauri/Cargo.toml et src-tauri/Cargo.lock.
//
//   npm run configurer             ecrit les fichiers qui different
//   npm run configurer -- --verifier   n'ecrit rien, sort en erreur si un fichier n'est pas a jour
//
// Le code Rust (build.rs) et l'interface (vite.config.ts) lisent launcher.config.json directement.
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const ICI = dirname(fileURLToPath(import.meta.url));
const LAUNCHER = join(ICI, "..");
const DEPOT = join(LAUNCHER, "..");
const CONFIG = join(DEPOT, "launcher.config.json");
const TAURI = join(LAUNCHER, "src-tauri", "tauri.conf.json");
const PACKAGE = join(LAUNCHER, "package.json");
const PACKAGE_LOCK = join(LAUNCHER, "package-lock.json");
const CARGO = join(LAUNCHER, "src-tauri", "Cargo.toml");
const CARGO_LOCK = join(LAUNCHER, "src-tauri", "Cargo.lock");
const PAQUET_CARGO = "arma-launcher";

const verifier = process.argv.includes("--verifier");
const erreurs = [];

function lireJson(chemin) {
  try {
    return JSON.parse(readFileSync(chemin, "utf8"));
  } catch (e) {
    console.error(`${relative(DEPOT, chemin)} illisible : ${e.message}`);
    process.exit(1);
  }
}

const c = lireJson(CONFIG);

function exiger(cle, test, message) {
  if (typeof c[cle] !== "string" || !test(c[cle])) erreurs.push(`"${cle}" : ${message}`);
}

exiger("nom", (v) => v.trim().length > 0 && !/[<>:"/\\|?*]/.test(v), "nom du produit requis, sans < > : \" / \\ | ? *");
exiger("version", (v) => /^\d+\.\d+\.\d+$/.test(v), "version attendue sous la forme 1.2.3");
exiger("identifiant", (v) => /^[a-zA-Z][a-zA-Z0-9-]*(\.[a-zA-Z][a-zA-Z0-9-]*)+$/.test(v), "identifiant inverse attendu, ex. com.monserveur.launcher");
exiger("dossierDonnees", (v) => v.trim().length > 0 && !/[<>:"/\\|?*]/.test(v), "nom de dossier Windows valide requis");
exiger("urlServeur", (v) => /^https?:\/\/[^\s/]+(\/[^\s]*)?$/.test(v) && !v.endsWith("/"), "URL complete sans / final, ex. https://play.example.com");
exiger("userAgent", (v) => /^[A-Za-z0-9._-]+$/.test(v), "lettres, chiffres, . _ - seulement");
if (c.clePubliqueMaj !== undefined && typeof c.clePubliqueMaj !== "string") erreurs.push(`"clePubliqueMaj" : texte attendu`);
if (typeof c.urlServeur === "string" && c.urlServeur.startsWith("http://")) {
  console.warn("Attention : urlServeur en http://, a reserver aux essais locaux (le launcher doit parler en https).");
}
if (erreurs.length) {
  console.error("launcher.config.json invalide :\n  " + erreurs.join("\n  "));
  process.exit(1);
}

const cle = (c.clePubliqueMaj ?? "").trim();
const changes = [];

function poser(chemin, ancien, nouveau) {
  if (ancien === nouveau) return;
  changes.push(relative(DEPOT, chemin));
  if (!verifier) writeFileSync(chemin, nouveau);
}

function finDeLigne(texte) {
  return texte.includes("\r\n") ? "\r\n" : "\n";
}

{
  const texte = readFileSync(TAURI, "utf8");
  const t = JSON.parse(texte);
  t.productName = c.nom;
  t.version = c.version;
  t.identifier = c.identifiant;
  for (const f of t.app.windows) f.title = c.nom;
  t.bundle.createUpdaterArtifacts = cle.length > 0;
  if (c.editeur) t.bundle.publisher = c.editeur;
  else delete t.bundle.publisher;
  t.plugins.updater.pubkey = cle;
  t.plugins.updater.endpoints = [`${c.urlServeur}/update/latest.json`];
  const fl = finDeLigne(texte);
  poser(TAURI, texte, JSON.stringify(t, null, 2).replace(/\n/g, fl) + fl);
}

{
  const texte = readFileSync(PACKAGE, "utf8");
  poser(PACKAGE, texte, texte.replace(/("version"\s*:\s*")[^"]*(")/, `$1${c.version}$2`));
}

{
  const texte = readFileSync(PACKAGE_LOCK, "utf8");
  poser(PACKAGE_LOCK, texte, texte.replace(/("name": "arma-launcher",\s*"version": ")[^"]*(")/g, `$1${c.version}$2`));
}

{
  const texte = readFileSync(CARGO, "utf8");
  poser(CARGO, texte, texte.replace(/(\[package\][^[]*?\nversion\s*=\s*")[^"]*(")/, `$1${c.version}$2`));
}

{
  const texte = readFileSync(CARGO_LOCK, "utf8");
  const motif = new RegExp(`(name = "${PAQUET_CARGO}"\\r?\\nversion = ")[^"]*(")`);
  poser(CARGO_LOCK, texte, texte.replace(motif, `$1${c.version}$2`));
}

if (!cle) console.warn("Attention : clePubliqueMaj vide, la mise a jour automatique du launcher est desactivee (voir docs/03-signer-et-construire.md).");

if (verifier) {
  if (changes.length) {
    console.error("Pas a jour, lancer npm run configurer : " + changes.join(", "));
    process.exit(1);
  }
  console.log("Configuration a jour.");
} else {
  console.log(changes.length ? "Mis a jour : " + changes.join(", ") : "Configuration deja a jour.");
}
