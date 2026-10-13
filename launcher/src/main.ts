import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getVersion } from "@tauri-apps/api/app";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open } from "@tauri-apps/plugin-dialog";
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { poserTopo } from "./topo";

type Reglages = { dossier_arma: string | null; dossier_mods: string; parametres: string; apres_lancement: "reduire" | "fermer"; rejoindre_teamspeak: boolean };
type Lien = { nom: string; url: string; icone: string; description?: string };
type News = { date: string; titre: string; texte: string; lien?: Lien };
type Launcher = {
  version: number;
  serveur: { nom: string; hote: string; port: number; motDePasse: string };
  mods: string[];
  parametres: string;
  liens: Lien[];
  news: News[];
  tfarPlugin: string;
};
type Accueil = { reglages: Reglages; launcher: Launcher | null; hors_ligne: boolean; erreur: string | null; arma_detecte: boolean };
type Resume = { fichiers: number; octets: number; a_supprimer: number; pret: boolean; hors_ligne: boolean; espace_manquant: number };
type Info = { nom: string; carte: string; joueurs: number; max: number };
type Garde = { raison: "libre" } | { raison: "redemarrage"; secondes: number } | { raison: "hors_ligne" };
type Veille = { info: Info | null; garde: Garde; reboot_dans: number | null };
type Controles = { teamspeak: boolean; tfar: "absent" | "perime" | "ok" };
type Progression = { fait: number; total: number };

const el = <T extends HTMLElement = HTMLElement>(id: string) => document.getElementById(id) as T;
const fenetre = getCurrentWindow();

function echelle() {
  const racine = document.documentElement;
  const facteur = Math.min(racine.clientWidth / 1100, racine.clientHeight / 680);
  document.body.style.zoom = String(facteur);
}

let accueil: Accueil;
let resume: Resume | null = null;
let occupe = false;
let verifEchouee = false;
let mesure = { t: 0, fait: 0 };
let debut = { t: 0, fait: 0 };
let reboot: { secondes: number; lu: number } | null = null;
let garde: Garde = { raison: "libre" };
let gardeLue = 0;
let arme = false;
let prochaineVeille = 0;

const ICONES: Record<string, string> = {
  teamspeak: '<path d="M4 13v-2a8 8 0 0 1 16 0v2"/><rect x="3" y="13" width="4" height="7" rx="1.5"/><rect x="17" y="13" width="4" height="7" rx="1.5"/>',
  discord: '<path d="M4 5h11a2 2 0 0 1 2 2v6a2 2 0 0 1-2 2H9l-4 3v-3H4a2 2 0 0 1-2-2V7a2 2 0 0 1 2-2z"/><path d="M19 9h1a2 2 0 0 1 2 2v5a2 2 0 0 1-2 2v2.5L17 18h-4"/>',
  site: '<circle cx="12" cy="12" r="9"/><path d="M3 12h18M12 3c2.5 2.6 3.8 5.6 3.8 9s-1.3 6.4-3.8 9c-2.5-2.6-3.8-5.6-3.8-9S9.5 5.6 12 3z"/>',
};
const SOUS_TITRES: Record<string, string> = {
  teamspeak: "Voix et radio en jeu",
  discord: "Annonces et communauté",
  site: "Site du serveur",
};

function icone(nom: string, classe: string): SVGSVGElement {
  const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
  svg.setAttribute("viewBox", "0 0 24 24");
  svg.setAttribute("aria-hidden", "true");
  svg.setAttribute("class", classe);
  svg.innerHTML = nom === "fleche" ? '<path d="M9 5l7 7-7 7"/>' : ICONES[nom] ?? ICONES.site;
  return svg;
}

function ouvrirLien(url: string) {
  invoke("ouvrir_lien", { url }).catch((e) => {
    if (String(e) === "teamspeak_absent") el("bandeau-teamspeak").hidden = false;
    else erreur(String(e));
  });
}

function duree(secondes: number): string {
  const m = Math.round(secondes / 60);
  if (m < 1) return "moins d'une minute";
  if (m < 60) return `${m} min`;
  return `${Math.floor(m / 60)} h ${String(m % 60).padStart(2, "0")}`;
}

function afficherReboot() {
  const ligne = el("ligne-reboot");
  ligne.hidden = !reboot;
  if (!reboot) return;
  const reste = Math.max(0, reboot.secondes - (performance.now() - reboot.lu) / 1000);
  const heure = new Date(Date.now() + reste * 1000).toLocaleTimeString("fr-FR", { hour: "2-digit", minute: "2-digit" });
  el("reboot").textContent = `${heure} · dans ${duree(reste)}`;
}

const TELECHARGER_TEAMSPEAK = "https://files.teamspeak-services.com/releases/client/3.6.2/TeamSpeak3-Client-win64-3.6.2.exe";

function retenu(): boolean {
  return garde.raison !== "libre";
}

function rebours(): string {
  if (garde.raison !== "redemarrage") return "";
  const s = Math.max(0, Math.round(garde.secondes - (performance.now() - gardeLue) / 1000));
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
}

function taille(n: number): string {
  if (n < 1024 * 1024) return `${Math.max(1, Math.round(n / 1024))} Ko`;
  if (n < 1024 ** 3) return `${(n / 1024 / 1024).toFixed(1).replace(".", ",")} Mo`;
  return `${(n / 1024 ** 3).toFixed(2).replace(".", ",")} Go`;
}

function erreur(message: string | null) {
  const zone = el("erreur");
  zone.hidden = !message;
  zone.textContent = message ?? "";
}

function jauge(pourcent: number | null, balaye = false) {
  const barre = el("jauge");
  barre.className = `jauge${balaye ? " balaye" : ""}`;
  barre.hidden = pourcent === null && !balaye;
  barre.style.width = balaye ? "100%" : `${(pourcent ?? 0).toFixed(1)}%`;
  const compteur = el("pourcent");
  compteur.hidden = pourcent === null;
  compteur.textContent = pourcent === null ? "" : `${Math.round(pourcent)} %`;
}

function rendreNews(news: News[]) {
  const zone = el("news");
  zone.replaceChildren();
  const triee = [...news].sort((a, b) => b.date.localeCompare(a.date));
  if (!triee.length) {
    const p = document.createElement("p");
    p.className = "vide";
    p.textContent = "Aucune actualité pour le moment.";
    zone.append(p);
    return;
  }
  const mois = ["janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.", "déc."];
  const aujourdhui = new Date();
  triee.forEach((n, i) => {
    const article = document.createElement("article");
    article.style.animationDelay = `${Math.min(i, 6) * 45}ms`;
    const [a, m, j] = n.date.split("-").map(Number);
    const date = document.createElement("time");
    date.dateTime = n.date;
    const jour = document.createElement("b");
    jour.textContent = j ? String(j) : n.date;
    const nomMois = document.createElement("span");
    nomMois.textContent = mois[m - 1] ?? "";
    date.append(jour, nomMois);
    const corps = document.createElement("div");
    const titre = document.createElement("h3");
    titre.textContent = n.titre;
    const age = (aujourdhui.getTime() - new Date(a, m - 1, j).getTime()) / 864e5;
    if (i === 0 && age < 7) {
      article.className = "recente";
      const badge = document.createElement("span");
      badge.className = "badge";
      badge.textContent = "Nouveau";
      titre.append(badge);
    }
    const texte = document.createElement("p");
    texte.textContent = n.texte;
    corps.append(titre, texte);
    if (n.lien) {
      const lien = n.lien;
      const bouton = document.createElement("button");
      bouton.className = "lien-news";
      bouton.textContent = lien.nom;
      bouton.append(icone("fleche", "fleche"));
      bouton.onclick = () => ouvrirLien(lien.url);
      corps.append(bouton);
    }
    article.append(date, corps);
    zone.append(article);
  });
}

function rendreLiens(liens: Lien[]) {
  const zone = el("liens");
  zone.replaceChildren();
  for (const lien of liens) {
    const type = lien.icone || "site";
    const bouton = document.createElement("button");
    bouton.className = `lien lien-${type}`;
    const texte = document.createElement("span");
    texte.className = "lien-texte";
    const nom = document.createElement("b");
    nom.textContent = lien.nom;
    const sous = document.createElement("small");
    sous.textContent = lien.description || SOUS_TITRES[type] || "";
    texte.append(nom, sous);
    bouton.append(icone(type, "lien-icone"), texte, icone("fleche", "fleche"));
    bouton.onclick = () => ouvrirLien(lien.url);
    zone.append(bouton);
  }
}

function rendre() {
  const l = accueil.launcher;
  el("titre-nom").textContent = l?.serveur.nom ?? __NOM_LAUNCHER__;
  el("ligne-teamspeak").hidden = !utiliseTeamspeak();
  el("adresse").textContent = l ? `${l.serveur.hote}:${l.serveur.port}` : "";
  el("titre-fiche").hidden = el("fiche").hidden = !l;
  el("hors-ligne").hidden = !accueil.hors_ligne;
  rendreNews(l?.news ?? []);
  rendreLiens(l?.liens ?? []);
  majBouton();
}

function poser(classe: string, libelle: string, actif: boolean, etat: string, detail = "") {
  const bouton = el<HTMLButtonElement>("principal");
  bouton.className = classe ? `principal ${classe}` : "principal";
  bouton.textContent = libelle;
  bouton.disabled = !actif;
  el("etat-mods").textContent = etat;
  el("detail").textContent = detail;
}

function majBouton() {
  el<HTMLButtonElement>("ouvrir-parametres").disabled = occupe;
  el("forcer").hidden = true;
  if (occupe) {
    el<HTMLButtonElement>("principal").disabled = true;
    return;
  }
  jauge(null);
  if (!accueil.launcher) {
    poser("", "Indisponible", false, "Configuration du serveur indisponible.", accueil.erreur ?? "");
    return;
  }
  if (!accueil.arma_detecte) {
    poser("config", "Configurer Arma 3", true, "Dossier Arma 3 introuvable.", "Indiquez où Arma 3 est installé.");
    return;
  }
  if (!resume) {
    if (verifEchouee) poser("maj", "Réessayer", true, "Vérification impossible.", "");
    else poser("", "Vérification…", false, "Vérification des mods…", "");
    return;
  }
  if (resume.pret && retenu()) {
    const etat = garde.raison === "redemarrage" ? `Le serveur redémarre dans ${rebours()}.` : "Le serveur est hors ligne.";
    const detail = arme
      ? "Le jeu se lancera tout seul dès que le serveur sera revenu."
      : garde.raison === "redemarrage"
        ? "Se connecter maintenant, c'est être éjecté dans un instant."
        : "Le launcher surveille son retour.";
    poser(arme ? "attente arme" : "attente", arme ? "Annuler" : "Lancer au retour", true, etat, detail);
    el("forcer").hidden = false;
    return;
  }
  if (resume.pret) {
    const mods = accueil.launcher.mods.length;
    const detail = accueil.hors_ligne ? "Hors ligne : les mods déjà installés seront utilisés." : `${mods} mods vérifiés.`;
    poser("jouer", "Jouer", true, "Prêt à jouer.", detail);
  } else if (accueil.hors_ligne) {
    poser("", "Mods absents", false, "Hors ligne et des mods manquent.", "Reconnectez-vous pour les télécharger.");
  } else {
    if (resume.espace_manquant > 0) {
      poser("", "Espace insuffisant", false, "Pas assez de place sur le disque des mods.", `Libérez ${taille(resume.espace_manquant)} ou changez le dossier des mods dans les paramètres.`);
      return;
    }
    const suppr = resume.a_supprimer ? ` · ${resume.a_supprimer} à supprimer` : "";
    poser("maj", "Mettre à jour", true, "Mise à jour des mods disponible.", `${resume.fichiers} fichier${resume.fichiers > 1 ? "s" : ""} · ${taille(resume.octets)}${suppr}`);
  }
}

async function verifier(complet = false) {
  if (occupe) return;
  resume = null;
  verifEchouee = false;
  occupe = true;
  majBouton();
  poser("", "Vérification…", false, complet ? "Vérification complète des fichiers…" : "Vérification des mods…", "");
  jauge(null, true);
  try {
    resume = await invoke<Resume>("verifier", { complet });
    accueil.hors_ligne = resume.hors_ligne;
    el("hors-ligne").hidden = !accueil.hors_ligne;
    erreur(null);
  } catch (e) {
    verifEchouee = true;
    erreur(String(e));
  } finally {
    occupe = false;
    majBouton();
  }
}

async function principal(forcer = false) {
  if (!accueil.arma_detecte) {
    ouvrirParametres();
    return;
  }
  if (occupe) return;
  if (!resume) {
    if (verifEchouee) await verifier();
    return;
  }
  if (resume.pret && retenu() && !forcer) {
    arme = !arme;
    majBouton();
    return;
  }
  arme = false;
  occupe = true;
  majBouton();
  try {
    if (resume.pret) {
      poser("jouer", "Lancement…", false, "Démarrage d'Arma 3…", "");
      await invoke("jouer");
    } else {
      mesure = { t: performance.now(), fait: 0 };
      debut = { ...mesure };
      poser("maj", "Téléchargement…", false, "Téléchargement des mods…", "");
      jauge(0);
      resume = await invoke<Resume>("mettre_a_jour");
    }
    erreur(null);
  } catch (e) {
    erreur(String(e));
  } finally {
    occupe = false;
    majBouton();
  }
}

async function statut() {
  const l = accueil?.launcher;
  if (!l) return;
  const veille = await invoke<Veille>("veille_serveur", { hote: l.serveur.hote, port: l.serveur.port }).catch(() => null);
  const info = veille?.info ?? null;
  const etaitRetenu = retenu();
  garde = veille?.garde ?? { raison: "libre" };
  reboot = veille?.reboot_dans ? { secondes: veille.reboot_dans, lu: performance.now() } : null;
  afficherReboot();
  gardeLue = performance.now();
  prochaineVeille = gardeLue + (retenu() ? 10000 : 30000);
  if (!occupe) majBouton();
  if (etaitRetenu && !retenu() && arme && resume?.pret) principal();
  el("pastille").className = `pastille ${info ? "en-ligne" : "hors-ligne"}`;
  el("etat-serveur").textContent = info ? "En ligne" : "Serveur hors ligne";
  el("joueurs").classList.remove("squelette");
  el("joueurs").textContent = info ? String(info.joueurs) : "—";
  el("joueurs-max").textContent = info ? `/${info.max}` : "";
  el("sous-compte").textContent = info ? "joueurs connectés" : "aucune réponse du serveur";
  if (info) el("carte").textContent = info.carte;
}

function utiliseTeamspeak(): boolean {
  const l = accueil?.launcher;
  return !!l && (!!l.tfarPlugin || l.liens.some((x) => x.url.toLowerCase().startsWith("ts3server://")));
}

async function controles() {
  const c = await invoke<Controles>("controles").catch((): Controles => ({ teamspeak: true, tfar: "ok" }));
  el("bandeau-teamspeak").hidden = c.teamspeak || !utiliseTeamspeak();
  el("bandeau-tfar").hidden = !c.teamspeak || c.tfar === "ok" || !accueil?.launcher?.tfarPlugin || accueil.hors_ligne;
  el("texte-tfar").textContent = c.tfar === "perime"
    ? "Votre plugin TFAR n'est pas celui du serveur : pas de voix ni de radio en jeu. Fermez TeamSpeak puis mettez-le à jour."
    : "Le plugin TeamSpeak TFAR manque : pas de voix ni de radio en jeu.";
  el("installer-tfar").textContent = c.tfar === "perime" ? "Mettre à jour" : "Installer";
}

function battement() {
  if (retenu() && !occupe) majBouton();
  afficherReboot();
  if (accueil?.launcher && performance.now() >= prochaineVeille) {
    prochaineVeille = performance.now() + 30000;
    statut();
  }
}

function erreurDialogue(message: string | null) {
  const zone = el("p-erreur");
  zone.hidden = !message;
  zone.textContent = message ?? "";
}

function ouvrirParametres() {
  if (occupe) return;
  const r = accueil.reglages;
  el<HTMLInputElement>("p-arma").value = r.dossier_arma ?? "";
  el<HTMLInputElement>("p-mods").value = r.dossier_mods;
  el<HTMLInputElement>("p-params").value = r.parametres;
  el<HTMLSelectElement>("p-apres").value = r.apres_lancement;
  el<HTMLInputElement>("p-teamspeak").checked = r.rejoindre_teamspeak;
  erreurDialogue(null);
  el<HTMLDialogElement>("parametres").showModal();
}

async function choisirDossier(champ: string) {
  const dossier = await open({ directory: true, defaultPath: el<HTMLInputElement>(champ).value || undefined });
  if (typeof dossier === "string") el<HTMLInputElement>(champ).value = dossier;
}

async function enregistrer() {
  if (occupe) {
    erreurDialogue("Opération en cours, patientez.");
    return;
  }
  const reglages: Reglages = {
    dossier_arma: el<HTMLInputElement>("p-arma").value.trim() || null,
    dossier_mods: el<HTMLInputElement>("p-mods").value.trim(),
    parametres: el<HTMLInputElement>("p-params").value.trim(),
    apres_lancement: el<HTMLSelectElement>("p-apres").value as Reglages["apres_lancement"],
    rejoindre_teamspeak: el<HTMLInputElement>("p-teamspeak").checked,
  };
  const deplace = reglages.dossier_mods.toLowerCase() !== accueil.reglages.dossier_mods.toLowerCase();
  const etat = el("p-etat");
  const boutons = ["enregistrer", "annuler", "integrite", "fermer-parametres", "choisir-arma", "choisir-mods"].map((id) => el<HTMLButtonElement>(id));
  erreurDialogue(null);
  if (deplace) {
    occupe = true;
    boutons.forEach((b) => (b.disabled = true));
    etat.hidden = false;
    etat.textContent = "Déplacement des mods vers le nouveau dossier…";
    mesure = { t: performance.now(), fait: 0 };
    debut = { ...mesure };
  }
  try {
    accueil = await invoke<Accueil>("sauver_reglages", { reglages });
    el<HTMLDialogElement>("parametres").close();
    erreur(null);
  } catch (e) {
    erreurDialogue(String(e));
  } finally {
    occupe = false;
    boutons.forEach((b) => (b.disabled = false));
    etat.hidden = true;
    jauge(null);
  }
  if (!el<HTMLDialogElement>("parametres").open) {
    rendre();
    if (accueil.launcher) await verifier();
  }
}

async function miseAJourLauncher() {
  if (!__MAJ_AUTO__) return;
  try {
    const maj = await check();
    if (!maj) return;
    el("maj-version").textContent = maj.version;
    const dialogue = el<HTMLDialogElement>("maj");
    el("maj-plus-tard").onclick = () => dialogue.close();
    el("maj-installer").onclick = async () => {
      if (occupe) {
        el("maj-etat").textContent = "Mise à jour des mods en cours, réessayez après.";
        return;
      }
      el<HTMLButtonElement>("maj-installer").disabled = true;
      el("maj-etat").textContent = "Téléchargement…";
      try {
        await maj.downloadAndInstall();
        await relaunch();
      } catch (e) {
        el("maj-etat").textContent = String(e);
        el<HTMLButtonElement>("maj-installer").disabled = false;
      }
    };
    dialogue.showModal();
  } catch {
    return;
  }
}

listen<Progression>("progression", ({ payload }) => {
  const pourcent = payload.total ? (payload.fait / payload.total) * 100 : 100;
  jauge(pourcent);
  const maintenant = performance.now();
  const secondes = (maintenant - mesure.t) / 1000;
  if (secondes >= 1) {
    const debit = Math.max(0, (payload.fait - mesure.fait) / secondes);
    const moyen = (payload.fait - debut.fait) / Math.max(1, (maintenant - debut.t) / 1000);
    const reste = moyen > 0 && maintenant - debut.t > 3000 ? ` · encore ${duree((payload.total - payload.fait) / moyen)}` : "";
    el("detail").textContent = `${taille(payload.fait)} / ${taille(payload.total)} · ${taille(debit)}/s${reste}`;
    const etat = el("p-etat");
    if (!etat.hidden) etat.textContent = `Déplacement des mods : ${taille(payload.fait)} / ${taille(payload.total)}${reste}`;
    mesure = { t: maintenant, fait: payload.fait };
  }
});

echelle();
window.addEventListener("resize", echelle);

el("reduire").onclick = () => fenetre.minimize();
el("fermer").onclick = () => fenetre.close();
el("principal").onclick = () => principal();
el("forcer").onclick = () => principal(true);
el("installer-teamspeak").onclick = () => ouvrirLien(TELECHARGER_TEAMSPEAK);
el("ouvrir-parametres").onclick = ouvrirParametres;
el("choisir-arma").onclick = () => choisirDossier("p-arma");
el("choisir-mods").onclick = () => choisirDossier("p-mods");
el("annuler").onclick = () => el<HTMLDialogElement>("parametres").close();
el("fermer-parametres").onclick = () => el<HTMLDialogElement>("parametres").close();
el("parametres").addEventListener("cancel", (e) => {
  if (occupe) e.preventDefault();
});
el("enregistrer").onclick = enregistrer;
el("integrite").onclick = async () => {
  if (occupe) {
    erreur("Opération en cours, patientez.");
    return;
  }
  el<HTMLDialogElement>("parametres").close();
  await verifier(true);
};
el("installer-tfar").onclick = () => invoke("tfar_installer").catch((e) => erreur(String(e)));
window.addEventListener("focus", controles);

async function demarrer() {
  el("titre-version").textContent = `v${await getVersion()}`;
  accueil = await invoke<Accueil>("charger");
  erreur(accueil.erreur);
  rendre();
  statut();
  setInterval(battement, 1000);
  controles();
  miseAJourLauncher();
  if (!accueil.arma_detecte) {
    ouvrirParametres();
    el<HTMLInputElement>("p-arma").focus();
  }
  if (accueil.launcher) await verifier();
}

demarrer().catch((e) => erreur(String(e)));
try {
  poserTopo(document.querySelector<HTMLElement>(".entete")!);
} catch {}
