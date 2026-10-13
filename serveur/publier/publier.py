# -*- coding: utf-8 -*-
"""Publie les mods, launcher.json, la mise a jour du launcher et la page d'accueil.

    docker compose run --rm publier [--a-sec] [--notes "texte"]

Dossiers (montes par docker-compose.yml, modifiables par variables d'environnement) :

    MODS            /mods             les dossiers @Mod a distribuer (+ un .ts3_plugin facultatif)
    CONFIG          /config           launcher.json edite a la main (serveur, liens, news)
    LAUNCHER_BUILD  /launcher-build   installeur NSIS *_x64-setup.exe et sa signature .sig (facultatif)
    DONNEES         /donnees          ce que nginx sert
    URL_PUBLIQUE                      https://play.example.com (requis pour publier le launcher)

Ordre de publication (le meme que l'outil d'origine) : fichiers de mods copies, puis
launcher.json, puis manifest.json remplace d'un coup, puis fichiers disparus supprimes.
Le manifeste en ligne ne cite donc jamais un fichier absent, ni un mod hors de la
liste en ligne (le launcher refuse le manifeste entier sur un seul chemin hors liste).

    --a-sec          affiche ce qui serait fait, n'ecrit rien
    --notes "texte"  notes de version de la mise a jour du launcher (update/latest.json)
"""
import argparse
import datetime
import hashlib
import html
import json
import os
import re
import shutil
import sys
import urllib.parse
import zipfile

MODS = os.environ.get("MODS", "/mods")
CONFIG = os.environ.get("CONFIG", "/config")
LAUNCHER_BUILD = os.environ.get("LAUNCHER_BUILD", "/launcher-build")
DONNEES = os.environ.get("DONNEES", "/donnees")
URL_PUBLIQUE = os.environ.get("URL_PUBLIQUE", "").strip().rstrip("/")

CACHE = os.path.join(DONNEES, ".publier", "cache.json")
TEMPORAIRE = ".publier-tmp"
INSTALLEUR_STABLE = "launcher-setup.exe"
SCHEMAS_LIENS = ("https://", "ts3server://", "discord://")
SETUP = re.compile(r"^(?P<nom>.+)_(?P<version>\d+\.\d+\.\d+)_x64-setup\.exe$")

PAGE = """<!doctype html>
<html lang="fr">
<head>
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
<title>%(nom)s - Launcher</title>
<style>
  :root {
    --fond: #0b0f12; --surface: #12181d; --trait: #233039;
    --texte: #eceeea; --discret: #8b98a1; --accent: #6cb8e6; --ok: #5fc98a;
    color-scheme: dark;
  }
  * { box-sizing: border-box; }
  body {
    margin: 0; min-height: 100vh; background: var(--fond); color: var(--texte);
    font: 16px/1.6 "Segoe UI", system-ui, sans-serif;
    display: flex; flex-direction: column; align-items: center;
  }
  header { width: 100%%; padding: 48px 16px 72px; text-align: center; border-bottom: 1px solid var(--trait); background: var(--surface); }
  h1 { margin: 0 0 8px; font-size: clamp(30px, 7vw, 46px); letter-spacing: .04em; }
  .accroche { margin: 0; color: var(--discret); }
  main { width: 100%%; max-width: 820px; padding: 0 16px 56px; }
  .telecharger {
    display: flex; flex-direction: column; align-items: center; gap: 10px; text-align: center;
    margin: -40px auto 40px; padding: 28px 20px; max-width: 520px;
    background: var(--surface); border: 1px solid var(--trait); border-radius: 12px;
  }
  .bouton {
    display: inline-block; padding: 16px 32px; border-radius: 8px;
    background: var(--accent); color: var(--fond); font-size: 18px; font-weight: 700;
    letter-spacing: .04em; text-decoration: none;
  }
  .bouton:hover { filter: brightness(1.1); }
  .version { color: var(--discret); font-size: 14px; }
  h2 { margin: 32px 0 12px; font-size: 13px; letter-spacing: .12em; text-transform: uppercase; color: var(--accent); }
  ol, ul { margin: 0; padding-left: 20px; }
  li { margin-bottom: 8px; }
  .liens { display: flex; flex-wrap: wrap; gap: 10px; }
  .lien {
    padding: 10px 18px; border-radius: 8px; text-decoration: none; color: var(--texte);
    background: var(--surface); border: 1px solid var(--trait); border-left: 3px solid var(--accent);
  }
  .lien:hover { border-color: var(--accent); }
  .vide { color: var(--discret); }
  code { background: var(--surface); border: 1px solid var(--trait); border-radius: 4px; padding: 2px 6px; font-size: 14px; overflow-wrap: anywhere; }
  footer { color: var(--discret); font-size: 13px; padding: 24px 16px; text-align: center; }
</style>
</head>
<body>
  <header>
    <h1>%(nom)s</h1>
    <p class="accroche">Le launcher installe les mods, les tient &agrave; jour et vous connecte au serveur.</p>
  </header>

  <main>
    <div class="telecharger">
%(telecharger)s
    </div>

    <h2>Installation</h2>
    <ol>
      <li>T&eacute;l&eacute;chargez puis lancez l'installeur (aucun droit administrateur n'est demand&eacute;).</li>
      <li>Ouvrez le launcher : il t&eacute;l&eacute;charge les mods du serveur (%(taille)s).</li>
      <li>Cliquez sur <strong>JOUER</strong> : Arma 3 d&eacute;marre et rejoint le serveur.</li>
    </ol>

    <h2>Avant de jouer</h2>
    <ul>
      <li>Arma 3 install&eacute; sur Steam.</li>
%(tfar)s    </ul>

    <h2>Liens</h2>
    <div class="liens">
%(liens)s
    </div>

    <h2>Connexion directe</h2>
    <p>Adresse du serveur : <code>%(adresse)s</code> (le launcher s'en charge tout seul).</p>
  </main>

  <footer>%(nom)s &middot; %(annee)s</footer>
</body>
</html>
"""


def sortir(message):
    print("ERREUR : " + message, file=sys.stderr)
    sys.exit(1)


def sha256(chemin):
    h = hashlib.sha256()
    with open(chemin, "rb") as f:
        for bloc in iter(lambda: f.read(1024 * 1024), b""):
            h.update(bloc)
    return h.hexdigest()


def chemin_valide(chemin, mods):
    if not chemin or chemin.startswith("/") or "\\" in chemin or ":" in chemin:
        return False
    segments = chemin.split("/")
    if len(segments) < 2 or any(s in ("", ".", "..") for s in segments):
        return False
    return segments[0] in mods


def relatif_sur(chemin):
    return bool(chemin) and not chemin.startswith("/") and "\\" not in chemin and ":" not in chemin \
        and all(s not in ("", ".", "..") for s in chemin.split("/"))


def lire_json(chemin):
    if not os.path.isfile(chemin):
        return None
    try:
        with open(chemin, encoding="utf-8") as f:
            return json.load(f)
    except ValueError as e:
        sortir("%s illisible : %s" % (chemin, e))


def taille_lisible(octets):
    if octets < 1024 * 1024:
        return "%d Ko" % max(1, round(octets / 1024))
    if octets < 1024 ** 3:
        return ("%.1f Mo" % (octets / 1024 / 1024)).replace(".", ",")
    return ("%.2f Go" % (octets / 1024 ** 3)).replace(".", ",")


# ---------- configuration ----------

def lire_config():
    chemin = os.path.join(CONFIG, "launcher.json")
    config = lire_json(chemin)
    if config is None:
        sortir("%s introuvable : copier config/launcher.exemple.json en config/launcher.json puis l'editer" % chemin)
    if config.get("version") != 1:
        sortir("launcher.json : \"version\": 1 attendu (version du format, pas du launcher)")
    serveur = config.get("serveur")
    if not isinstance(serveur, dict) or not serveur.get("nom") or not serveur.get("hote"):
        sortir("launcher.json : bloc \"serveur\" avec \"nom\" et \"hote\" requis")
    if not isinstance(serveur.get("port"), int) or not 0 < serveur["port"] < 65536:
        sortir("launcher.json : serveur.port doit etre un entier (port de jeu, ex. 2302)")
    for lien in config.get("liens", []) + [n["lien"] for n in config.get("news", []) if n.get("lien")]:
        if not str(lien.get("url", "")).lower().startswith(SCHEMAS_LIENS):
            print("  [!] lien ignore par le launcher (schema refuse) : %s" % lien.get("url"))
    for n in config.get("news", []):
        if not (n.get("date") and n.get("titre") and "texte" in n):
            sortir("launcher.json : chaque news demande date, titre et texte : %r" % n)
    if "mods" in config and not (isinstance(config["mods"], list) and all(isinstance(m, str) for m in config["mods"])):
        sortir("launcher.json : \"mods\" (facultatif) doit etre une liste de noms @Mod, utilisee pour l'ordre de chargement")
    etat = config.get("etatServeur", "")
    if etat and not relatif_sur(etat.split("?")[0]):
        sortir("launcher.json : etatServeur doit etre un chemin relatif (ex. api/serveur/etat)")
    return config


def lister_mods(preference):
    if not os.path.isdir(MODS):
        sortir("dossier des mods absent : %s" % MODS)
    presents = sorted(n for n in os.listdir(MODS) if n.startswith("@") and os.path.isdir(os.path.join(MODS, n)))
    if not presents:
        sortir("aucun dossier @Mod dans %s" % MODS)
    for n in presents:
        if "/" in n or n in ("@.", "@.."):
            sortir("nom de mod refuse : %s" % n)
    ordre = [m for m in (preference or []) if m in presents]
    for m in preference or []:
        if m not in presents:
            print("  [!] mod de \"mods\" (ordre) absent de %s, ignore : %s" % (MODS, m))
    return ordre + [m for m in presents if m not in ordre]


def plugin_tfar():
    plugins = sorted(n for n in os.listdir(MODS) if n.lower().endswith(".ts3_plugin") and os.path.isfile(os.path.join(MODS, n)))
    if len(plugins) > 1:
        print("  [!] plusieurs .ts3_plugin dans %s, seul %s est publie" % (MODS, plugins[0]))
    return os.path.join(MODS, plugins[0]) if plugins else None


def empreinte_dll_tfar(plugin):
    try:
        with zipfile.ZipFile(plugin) as z:
            return hashlib.sha256(z.read("plugins/TFAR_win64.dll")).hexdigest()
    except (KeyError, zipfile.BadZipFile):
        return ""


def config_publiee(config, mods, tfar):
    publiee = {}
    for cle, valeur in config.items():
        if cle in ("mods", "tfarPlugin", "tfarDll"):
            continue
        publiee[cle] = valeur
        if cle == "serveur":
            publiee["mods"] = mods
    publiee.setdefault("mods", mods)
    if tfar:
        publiee["tfarPlugin"] = "tfar/" + os.path.basename(tfar)
        dll = empreinte_dll_tfar(tfar)
        if dll:
            publiee["tfarDll"] = dll
    return publiee


# ---------- manifeste ----------

def charger_cache():
    return lire_json(CACHE) or {}


def calculer_manifeste(mods, genere, cache):
    fichiers = []
    nouveau_cache = {}
    for mod in mods:
        for dossier, sous, noms in os.walk(os.path.join(MODS, mod)):
            sous.sort()
            for nom in sorted(noms):
                plein = os.path.join(dossier, nom)
                rel = os.path.relpath(plein, MODS).replace(os.sep, "/")
                if not chemin_valide(rel, mods):
                    sortir("chemin refuse : %s" % rel)
                st = os.stat(plein)
                cle = "%d:%d" % (st.st_size, st.st_mtime_ns)
                connu = cache.get(rel)
                sha = connu[1] if connu and connu[0] == cle else sha256(plein)
                nouveau_cache[rel] = [cle, sha]
                fichiers.append({"chemin": rel, "taille": st.st_size, "sha256": sha})
    fichiers.sort(key=lambda f: f["chemin"])
    return {"genere": genere, "fichiers": fichiers}, nouveau_cache


def delta(nouveau, ancien):
    anciens = {f["chemin"]: f for f in (ancien or {}).get("fichiers", [])}
    nouveaux = {f["chemin"] for f in nouveau["fichiers"]}
    envoyer = []
    for f in nouveau["fichiers"]:
        cible = os.path.join(DONNEES, "mods", *f["chemin"].split("/"))
        if anciens.get(f["chemin"]) != f or not os.path.isfile(cible) or os.path.getsize(cible) != f["taille"]:
            envoyer.append(f)
    supprimer = sorted(c for c in anciens if c not in nouveaux)
    return envoyer, supprimer


# ---------- ecriture ----------

def copier_atomique(source, cible):
    os.makedirs(os.path.dirname(cible), exist_ok=True)
    tmp = cible + TEMPORAIRE
    shutil.copyfile(source, tmp)
    os.chmod(tmp, 0o644)
    os.replace(tmp, cible)


def ecrire_atomique(contenu, cible):
    os.makedirs(os.path.dirname(cible), exist_ok=True)
    tmp = cible + TEMPORAIRE
    with open(tmp, "w", encoding="utf-8", newline="\n") as f:
        f.write(contenu)
    os.chmod(tmp, 0o644)
    os.replace(tmp, cible)


def json_texte(donnees):
    return json.dumps(donnees, ensure_ascii=False, indent=1) + "\n"


def supprimer_fichiers(chemins):
    racine = os.path.join(DONNEES, "mods")
    for c in chemins:
        try:
            os.remove(os.path.join(racine, *c.split("/")))
        except FileNotFoundError:
            pass
    for dossier, _, _ in sorted(os.walk(racine), key=lambda e: -len(e[0])):
        if dossier != racine and not os.listdir(dossier):
            os.rmdir(dossier)


def nettoyer_temporaires():
    for dossier, _, noms in os.walk(DONNEES):
        for nom in noms:
            if nom.endswith(TEMPORAIRE):
                os.remove(os.path.join(dossier, nom))


# ---------- launcher ----------

def cle_version(v):
    return tuple(int(x) for x in v.split("."))


def installeur_local():
    if not os.path.isdir(LAUNCHER_BUILD):
        return None
    trouves = []
    for nom in os.listdir(LAUNCHER_BUILD):
        m = SETUP.match(nom)
        if m and os.path.isfile(os.path.join(LAUNCHER_BUILD, nom + ".sig")):
            trouves.append((cle_version(m.group("version")), nom, m.group("nom"), m.group("version")))
    if not trouves:
        return None
    _, nom, produit, version = max(trouves)
    return {"fichier": os.path.join(LAUNCHER_BUILD, nom), "produit": produit, "version": version}


def preparer_launcher(installeur, notes):
    if not installeur:
        return None
    if not URL_PUBLIQUE.startswith(("https://", "http://")):
        sortir("URL_PUBLIQUE manquante (fichier .env) : necessaire pour update/latest.json")
    with open(installeur["fichier"] + ".sig", encoding="utf-8") as f:
        signature = f.read().strip()
    nom = "%s_%s_x64-setup.exe" % (re.sub(r"[^A-Za-z0-9._-]+", "-", installeur["produit"]).strip("-") or "launcher", installeur["version"])
    actuel = lire_json(os.path.join(DONNEES, "update", "latest.json")) or {}
    plateforme = (actuel.get("platforms") or {}).get("windows-x86_64") or {}
    if actuel.get("version") == installeur["version"] and plateforme.get("signature") == signature \
            and os.path.isfile(os.path.join(DONNEES, "update", nom)):
        return {"version": installeur["version"], "nom": nom, "a_publier": False}
    latest = {
        "version": installeur["version"],
        "notes": notes or "",
        "pub_date": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "platforms": {"windows-x86_64": {"signature": signature, "url": "%s/update/%s" % (URL_PUBLIQUE, urllib.parse.quote(nom))}},
    }
    return {"version": installeur["version"], "nom": nom, "a_publier": True, "source": installeur["fichier"], "latest": latest}


def publier_launcher(maj):
    update = os.path.join(DONNEES, "update")
    copier_atomique(maj["source"], os.path.join(update, maj["nom"]))
    copier_atomique(maj["source"], os.path.join(update, INSTALLEUR_STABLE))
    ecrire_atomique(json_texte(maj["latest"]), os.path.join(update, "latest.json"))
    for nom in os.listdir(update):
        if SETUP.match(nom) and nom != maj["nom"]:
            os.remove(os.path.join(update, nom))


# ---------- page d'accueil ----------

def page_accueil(config, version, octets, tfar):
    e = html.escape
    serveur = config.get("serveur", {})
    nom = e(serveur.get("nom", "Serveur"))
    liens = [l for l in config.get("liens", []) if str(l.get("url", "")).lower().startswith(SCHEMAS_LIENS)]
    boutons = "\n".join('      <a class="lien" href="%s">%s</a>' % (e(l["url"], quote=True), e(l.get("nom", "Lien"))) for l in liens)
    if version:
        telecharger = ('      <a class="bouton" href="/update/%s">T&Eacute;L&Eacute;CHARGER LE LAUNCHER</a>\n'
                       '      <span class="version">Windows 64 bits &middot; version %s &middot; mise &agrave; jour automatique</span>'
                       % (INSTALLEUR_STABLE, e(version)))
    else:
        telecharger = '      <span class="version">Le launcher n\'est pas encore publi&eacute;.</span>'
    ts = '      <li>TeamSpeak 3 pour la radio : le launcher propose d\'installer le plugin TFAR s\'il manque.</li>\n' if tfar else ""
    return PAGE % {
        "nom": nom,
        "telecharger": telecharger,
        "taille": "environ " + taille_lisible(octets) + " au premier lancement",
        "tfar": ts,
        "liens": boutons or '      <span class="vide">Aucun lien</span>',
        "adresse": e("%s:%s" % (serveur.get("hote", ""), serveur.get("port", ""))),
        "annee": datetime.date.today().year,
    }


# ---------- programme ----------

def main(argv):
    p = argparse.ArgumentParser(description="Publie mods, launcher.json, mise a jour du launcher et page d'accueil.")
    p.add_argument("--a-sec", action="store_true", help="affiche ce qui serait fait, n'ecrit rien")
    p.add_argument("--notes", default="", help="notes de version du launcher (update/latest.json)")
    args = p.parse_args(argv)

    config = lire_config()
    mods = lister_mods(config.get("mods"))
    tfar = plugin_tfar()
    publiee = config_publiee(config, mods, tfar)

    maintenant = datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    cache = charger_cache()
    manifeste, nouveau_cache = calculer_manifeste(mods, maintenant, cache)
    ancien = lire_json(os.path.join(DONNEES, "manifest.json"))
    envoyer, supprimer = delta(manifeste, ancien)

    tfar_cible = os.path.join(DONNEES, "tfar", os.path.basename(tfar)) if tfar else None
    tfar_change = bool(tfar) and (not os.path.isfile(tfar_cible) or sha256(tfar_cible) != sha256(tfar))

    ancien_launcher = lire_json(os.path.join(DONNEES, "launcher.json"))
    launcher_change = ancien_launcher != publiee
    maj = preparer_launcher(installeur_local(), args.notes)

    octets = sum(f["taille"] for f in envoyer)
    total = sum(f["taille"] for f in manifeste["fichiers"])
    print("Publication dans %s%s" % (DONNEES, (" (" + URL_PUBLIQUE + ")") if URL_PUBLIQUE else ""))
    print("  mods : %s" % ", ".join(mods))
    print("  %d fichier(s) dans le manifeste, %s au total" % (len(manifeste["fichiers"]), taille_lisible(total)))
    print("  %d fichier(s) a copier (%s)" % (len(envoyer), taille_lisible(octets) if envoyer else "0"))
    for f in envoyer:
        print("    + %s" % f["chemin"])
    print("  %d fichier(s) a supprimer" % len(supprimer))
    for c in supprimer:
        print("    - %s" % c)
    print("  plugin TFAR : %s" % (("a copier : " if tfar_change else "inchange : ") + os.path.basename(tfar) if tfar else "aucun"))
    print("  launcher.json : %s" % ("a ecrire" if launcher_change else "inchange"))
    if maj:
        print("  launcher : version %s %s" % (maj["version"], "a publier" if maj["a_publier"] else "deja publiee"))
    else:
        print("  launcher : aucun installeur signe dans %s" % LAUNCHER_BUILD)
    if args.a_sec:
        print("--a-sec : rien n'a ete ecrit.")
        return 0

    os.makedirs(DONNEES, exist_ok=True)
    nettoyer_temporaires()
    for f in envoyer:
        copier_atomique(os.path.join(MODS, *f["chemin"].split("/")), os.path.join(DONNEES, "mods", *f["chemin"].split("/")))
    if tfar_change:
        copier_atomique(tfar, tfar_cible)
    if launcher_change:
        ecrire_atomique(json_texte(publiee), os.path.join(DONNEES, "launcher.json"))
    if envoyer or supprimer or ancien is None:
        ecrire_atomique(json_texte(manifeste), os.path.join(DONNEES, "manifest.json"))
    if supprimer:
        supprimer_fichiers(supprimer)
    tfar_dossier = os.path.join(DONNEES, "tfar")
    if os.path.isdir(tfar_dossier):
        for nom in os.listdir(tfar_dossier):
            if not tfar or nom != os.path.basename(tfar):
                os.remove(os.path.join(tfar_dossier, nom))
    if maj and maj["a_publier"]:
        publier_launcher(maj)
    latest = lire_json(os.path.join(DONNEES, "update", "latest.json")) or {}
    version = latest.get("version") if os.path.isfile(os.path.join(DONNEES, "update", INSTALLEUR_STABLE)) else None
    ecrire_atomique(page_accueil(publiee, version, total, tfar), os.path.join(DONNEES, "index.html"))
    ecrire_atomique(json.dumps(nouveau_cache), CACHE)
    print("Termine.")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
