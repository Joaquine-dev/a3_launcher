# -*- coding: utf-8 -*-
"""Publie les mods, launcher.json, la mise a jour du launcher et la page d'accueil.

    docker compose run --rm publier [--a-sec] [--notes "texte"]

Dossiers (montes par docker-compose.yml, modifiables par variables d'environnement) :

    MODS            /mods             les dossiers @Mod a distribuer (TFAR y livre son .ts3_plugin)
    CONFIG          /config           launcher.json edite a la main (serveur, liens, news)
    LAUNCHER_BUILD  /launcher-build   installeur NSIS *_x64-setup.exe et sa signature .sig (facultatif)
    DONNEES         /donnees          ce que nginx sert
    URL_PUBLIQUE                      https://play.example.com (requis pour publier le launcher)

Ordre de publication (le meme que l'outil d'origine) : fichiers de mods copies, puis
launcher.json, puis manifest.json remplace d'un coup, puis fichiers disparus supprimes
(tout fichier de donnees/mods absent du manifeste, ce qui rattrape une publication
interrompue). Le manifeste en ligne ne cite donc jamais un fichier absent ; le launcher
relit launcher.json avant chaque manifest.json.

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
INTERDITS_WINDOWS = re.compile(r'[<>"|?*\x00-\x1f]|[. ](/|$)')

PAGE = """<!doctype html>
<html lang="fr">
<head>
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
<title>%(nom)s - Launcher</title>
<style>
  :root {
    --fond: #141618; --surface: #1c1f22; --relief: #262a2e; --trait: rgb(255 255 255 / .08);
    --texte: #e4e2da; --discret: #969a91; --accent: #8a9a5b; --sur-accent: #101210;
    --corps: "IBM Plex Sans", "Segoe UI", system-ui, sans-serif;
    --donnees: "IBM Plex Mono", "Cascadia Mono", Consolas, ui-monospace, monospace;
    color-scheme: dark;
  }
  * { box-sizing: border-box; }
  body { margin: 0; min-height: 100vh; background: var(--fond); color: var(--texte); font: 16px/1.6 var(--corps); }
  .cadre { width: 100%%; max-width: 920px; margin: 0 auto; padding: 0 16px; }
  .etiquette { font: 500 12px/1.3 var(--donnees); letter-spacing: .14em; text-transform: uppercase; color: var(--discret); margin: 0; }
  header { border-bottom: 1px solid var(--trait); background: var(--surface); }
  header .cadre { padding-top: 56px; padding-bottom: 48px; }
  h1 { margin: 10px 0 8px; font-size: clamp(32px, 7vw, 52px); font-weight: 600; line-height: 1.1; letter-spacing: -.01em; overflow-wrap: anywhere; }
  .accroche { margin: 0; color: var(--discret); max-width: 560px; }
  .telecharger { display: flex; flex-wrap: wrap; align-items: center; gap: 14px 22px; margin-top: 32px; }
  .bouton {
    display: inline-block; padding: 18px 34px; background: var(--accent); color: var(--sur-accent);
    font-weight: 600; letter-spacing: .18em; text-transform: uppercase; text-decoration: none;
    clip-path: polygon(0 0, calc(100%% - 14px) 0, 100%% 14px, 100%% 100%%, 14px 100%%, 0 calc(100%% - 14px));
  }
  .bouton:hover { filter: brightness(1.1); }
  .bouton:focus-visible, a:focus-visible { outline: 2px solid var(--accent); outline-offset: 3px; }
  .version { font: 13px/1.5 var(--donnees); color: var(--discret); }
  main .cadre { display: grid; grid-template-columns: repeat(auto-fit, minmax(min(100%%, 400px), 1fr)); gap: 16px; padding-top: 32px; padding-bottom: 56px; }
  section { background: var(--surface); border: 1px solid var(--trait); border-radius: 4px; padding: 20px 22px; }
  section h2 { margin: 0 0 14px; }
  ol, ul { margin: 0; padding-left: 20px; }
  li { margin-bottom: 8px; }
  li::marker { color: var(--accent); font-family: var(--donnees); }
  .liens { display: grid; gap: 8px; }
  .lien { display: block; padding: 10px 14px; text-decoration: none; color: var(--texte); background: var(--relief); border: 1px solid transparent; border-radius: 4px; }
  .lien:hover { border-color: var(--accent); }
  .vide { color: var(--discret); }
  code { font: 15px/1.5 var(--donnees); color: var(--accent); overflow-wrap: anywhere; }
  footer { border-top: 1px solid var(--trait); }
  footer .cadre { color: var(--discret); font: 13px/1.5 var(--donnees); padding-top: 20px; padding-bottom: 28px; }
</style>
</head>
<body>
  <header>
    <div class="cadre">
      <p class="etiquette">Launcher officiel</p>
      <h1>%(nom)s</h1>
      <p class="accroche">Le launcher installe les mods du serveur, les tient &agrave; jour et lance Arma 3 directement sur le serveur.</p>
      <div class="telecharger">
%(telecharger)s
      </div>
    </div>
  </header>

  <main>
    <div class="cadre">
      <section>
        <h2 class="etiquette">Installation</h2>
        <ol>
          <li>T&eacute;l&eacute;chargez puis lancez l'installeur (aucun droit administrateur n'est demand&eacute;).</li>
          <li>Ouvrez le launcher : il t&eacute;l&eacute;charge les mods du serveur (%(taille)s).</li>
          <li>Cliquez sur <strong>JOUER</strong> : Arma 3 d&eacute;marre et rejoint le serveur.</li>
        </ol>
      </section>

      <section>
        <h2 class="etiquette">Avant de jouer</h2>
        <ul>
          <li>Arma 3 install&eacute; sur Steam.</li>
%(tfar)s        </ul>
      </section>

      <section>
        <h2 class="etiquette">Liens</h2>
        <div class="liens">
%(liens)s
        </div>
      </section>

      <section>
        <h2 class="etiquette">Connexion directe</h2>
        <p><code>%(adresse)s</code></p>
        <p class="vide">Le launcher s'en charge tout seul ; cette adresse sert pour rejoindre le serveur depuis Arma.</p>
      </section>
    </div>
  </main>

  <footer><div class="cadre">%(nom)s &middot; %(annee)s</div></footer>
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
    for n in sorted(os.listdir(MODS)):
        if n.startswith("@") and os.path.islink(os.path.join(MODS, n)) and not os.path.isdir(os.path.join(MODS, n)):
            print("  [!] lien symbolique sans cible dans le conteneur, ignore : %s (copiez le dossier lui-meme)" % n)
    presents = sorted(n for n in os.listdir(MODS) if n.startswith("@") and os.path.isdir(os.path.join(MODS, n)))
    if not presents:
        sortir("aucun dossier @Mod dans %s" % MODS)
    for n in presents:
        if not relatif_sur(n) or "/" in n or INTERDITS_WINDOWS.search(n):
            sortir("nom de mod refuse : %s" % n)
    ordre = [m for m in (preference or []) if m in presents]
    for m in preference or []:
        if m not in presents:
            print("  [!] mod de \"mods\" (ordre) absent de %s, ignore : %s" % (MODS, m))
    return ordre + [m for m in presents if m not in ordre]


DLL_TFAR = {"stable": "plugins/task_force_radio_win64.dll", "beta": "plugins/TFAR_win64.dll"}
VERSIONS_TFAR = ("stable", "beta")


def lire_tfar(config):
    """Bloc "tfar" de config/launcher.json : None si TFAR n'est pas publie, sinon {actif, version, plugin, ancien}.
    Sans bloc (ancien format) : un .ts3_plugin pose a la racine de mods/ est encore publie."""
    bloc = config.get("tfar")
    if bloc is None:
        return {"actif": True, "version": "", "plugin": "", "ancien": True}
    if not isinstance(bloc, dict):
        sortir("launcher.json : \"tfar\" doit etre un objet { \"actif\", \"version\", \"plugin\" }")
    if not isinstance(bloc.get("actif", False), bool):
        sortir("launcher.json : tfar.actif doit valoir true ou false")
    if not bloc.get("actif"):
        return None
    version = bloc.get("version", "stable")
    if version not in VERSIONS_TFAR:
        sortir("launcher.json : tfar.version doit valoir \"stable\" (TFAR 0.9.x) ou \"beta\" (TFAR 1.0)")
    plugin = bloc.get("plugin", "") or ""
    if not isinstance(plugin, str) or (plugin and not relatif_sur(plugin)):
        sortir("launcher.json : tfar.plugin doit etre vide (detection automatique) ou un chemin relatif a mods/ ou config/")
    return {"actif": True, "version": version, "plugin": plugin, "ancien": False}


def plugins_dans_les_mods(mods):
    trouves = []
    for mod in mods:
        for dossier, sous, noms in os.walk(os.path.join(MODS, mod)):
            sous[:] = sorted(n for n in sous if not ignore(n))
            for nom in sorted(noms):
                if nom.lower().endswith(".ts3_plugin") and not ignore(nom):
                    trouves.append(os.path.join(dossier, nom))
    return trouves


def plugin_tfar(reglage, mods):
    """Chemin du .ts3_plugin a publier, ou None."""
    if reglage is None:
        return None
    if reglage["ancien"]:
        plugins = sorted(n for n in os.listdir(MODS) if n.lower().endswith(".ts3_plugin") and os.path.isfile(os.path.join(MODS, n)))
        if len(plugins) > 1:
            print("  [!] plusieurs .ts3_plugin dans %s, seul %s est publie" % (MODS, plugins[0]))
        if plugins:
            print("  [!] plugin TFAR a la racine de mods/ : ajoutez plutot un bloc \"tfar\" a config/launcher.json (voir launcher.exemple.json)")
        return os.path.join(MODS, plugins[0]) if plugins else None
    if reglage["plugin"]:
        for base in (MODS, CONFIG):
            chemin = os.path.join(base, *reglage["plugin"].split("/"))
            if os.path.isfile(chemin):
                return chemin
        sortir("tfar.plugin introuvable dans mods/ ni dans config/ : %s" % reglage["plugin"])
    trouves = plugins_dans_les_mods(mods)
    if not trouves:
        sortir("tfar.actif vaut true mais aucun .ts3_plugin dans les mods (TFAR le livre dans @.../teamspeak/) : "
               "deposez le mod TFAR ou indiquez tfar.plugin")
    distincts = {}
    for chemin in trouves:
        distincts.setdefault(sha256(chemin), chemin)
    if len(distincts) > 1:
        sortir("plusieurs plugins TFAR differents dans les mods, precisez lequel dans tfar.plugin :\n    "
               + "\n    ".join(os.path.relpath(c, MODS).replace(os.sep, "/") for c in trouves))
    return trouves[0]


def nom_tfar(plugin):
    return re.sub(r"[^A-Za-z0-9._-]+", "-", os.path.basename(plugin)).lstrip(".-") or "task_force_radio.ts3_plugin"


def analyser_plugin_tfar(plugin):
    """(version, sha256 de la DLL 64 bits) lus dans le .ts3_plugin (une archive zip) ; ("", "") si illisible."""
    try:
        with zipfile.ZipFile(plugin) as z:
            noms = set(z.namelist())
            for version in VERSIONS_TFAR:
                if DLL_TFAR[version] in noms:
                    return version, hashlib.sha256(z.read(DLL_TFAR[version])).hexdigest()
    except (OSError, zipfile.BadZipFile):
        pass
    return "", ""


def infos_tfar(reglage, plugin):
    if not plugin:
        return None
    version, dll = analyser_plugin_tfar(plugin)
    if not version:
        print("  [!] %s : ni %s ni %s dans l'archive, le launcher ne pourra pas verifier le plugin du joueur"
              % (os.path.basename(plugin), DLL_TFAR["stable"], DLL_TFAR["beta"]))
    elif reglage["version"] and version != reglage["version"]:
        print("  [!] tfar.version vaut \"%s\" mais le plugin trouve est la version %s : c'est %s qui est publie"
              % (reglage["version"], version, version))
    return {"chemin": plugin, "nom": nom_tfar(plugin), "version": version or reglage["version"], "dll": dll}


def config_publiee(config, mods, tfar):
    publiee = {}
    for cle, valeur in config.items():
        if cle in ("mods", "tfar", "tfarPlugin", "tfarDll", "tfarVersion"):
            continue
        publiee[cle] = valeur
        if cle == "serveur":
            publiee["mods"] = mods
    publiee.setdefault("mods", mods)
    if tfar:
        publiee["tfarPlugin"] = "tfar/" + tfar["nom"]
        if tfar["dll"]:
            publiee["tfarDll"] = tfar["dll"]
        if tfar["version"]:
            publiee["tfarVersion"] = tfar["version"]
    return publiee


# ---------- manifeste ----------

def charger_cache():
    return lire_json(CACHE) or {}


def ignore(nom):
    return nom.startswith(".") or nom.endswith(".part") or nom.endswith(TEMPORAIRE)


def calculer_manifeste(mods, genere, cache):
    fichiers = []
    nouveau_cache = {}
    vus = {}
    for mod in mods:
        for dossier, sous, noms in os.walk(os.path.join(MODS, mod)):
            for nom in list(sous):
                plein = os.path.join(dossier, nom)
                if os.path.islink(plein):
                    sortir("lien symbolique refuse : %s (copiez les fichiers eux-memes)" % os.path.relpath(plein, MODS))
                if ignore(nom):
                    print("  [!] dossier ignore (cache, non servi par nginx) : %s" % os.path.relpath(plein, MODS))
                    sous.remove(nom)
            sous.sort()
            for nom in sorted(noms):
                plein = os.path.join(dossier, nom)
                rel = os.path.relpath(plein, MODS).replace(os.sep, "/")
                if os.path.islink(plein):
                    sortir("lien symbolique refuse : %s (copiez le fichier lui-meme)" % rel)
                if ignore(nom):
                    print("  [!] fichier ignore (cache, .part ou temporaire) : %s" % rel)
                    continue
                if not chemin_valide(rel, mods) or INTERDITS_WINDOWS.search(rel):
                    sortir("chemin refuse : %s" % rel)
                if rel.lower() in vus:
                    sortir("deux fichiers ne different que par la casse (conflit sous Windows) : %s et %s" % (vus[rel.lower()], rel))
                vus[rel.lower()] = rel
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
    racine = os.path.join(DONNEES, "mods")
    en_ligne = set(anciens)
    for dossier, _, noms in os.walk(racine):
        for nom in noms:
            if not nom.endswith(TEMPORAIRE):
                en_ligne.add(os.path.relpath(os.path.join(dossier, nom), racine).replace(os.sep, "/"))
    supprimer = sorted(c for c in en_ligne if c not in nouveaux)
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
        telecharger = ('      <a class="bouton" href="/update/%s">T&eacute;l&eacute;charger le launcher</a>\n'
                       '      <span class="version">Windows 64 bits &middot; version %s &middot; mise &agrave; jour automatique</span>'
                       % (INSTALLEUR_STABLE, e(version)))
    else:
        telecharger = '      <span class="version">Le launcher n\'est pas encore publi&eacute;.</span>'
    libelle = {"stable": " (version stable 0.9)", "beta": " (version 1.0 b&ecirc;ta)"}.get((tfar or {}).get("version"), "")
    ts = ('          <li>TeamSpeak 3 en 64 bits (pas TeamSpeak 5 ni 6) avec le plugin radio TFAR%s : '
          'le launcher propose de l\'installer s\'il manque.</li>\n' % libelle) if tfar else ""
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
    reglage_tfar = lire_tfar(config)
    tfar = infos_tfar(reglage_tfar, plugin_tfar(reglage_tfar, mods))
    publiee = config_publiee(config, mods, tfar)

    maintenant = datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    cache = charger_cache()
    manifeste, nouveau_cache = calculer_manifeste(mods, maintenant, cache)
    ancien = lire_json(os.path.join(DONNEES, "manifest.json"))
    envoyer, supprimer = delta(manifeste, ancien)

    tfar_cible = os.path.join(DONNEES, "tfar", tfar["nom"]) if tfar else None
    tfar_change = bool(tfar) and (not os.path.isfile(tfar_cible) or sha256(tfar_cible) != sha256(tfar["chemin"]))

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
    if tfar:
        print("  plugin TFAR (%s) : %s%s" % (tfar["version"] or "version inconnue", "a copier : " if tfar_change else "inchange : ",
                                           os.path.relpath(tfar["chemin"], MODS).replace(os.sep, "/")))
    else:
        print("  plugin TFAR : aucun")
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
        copier_atomique(tfar["chemin"], tfar_cible)
    if launcher_change:
        ecrire_atomique(json_texte(publiee), os.path.join(DONNEES, "launcher.json"))
    if envoyer or supprimer or ancien is None:
        ecrire_atomique(json_texte(manifeste), os.path.join(DONNEES, "manifest.json"))
    if supprimer:
        supprimer_fichiers(supprimer)
    tfar_dossier = os.path.join(DONNEES, "tfar")
    if os.path.isdir(tfar_dossier):
        for nom in os.listdir(tfar_dossier):
            if not tfar or nom != tfar["nom"]:
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
