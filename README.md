# Arma Launcher

Un launcher Windows pour **votre** serveur Arma 3, et le serveur de mises à jour qui va avec.

Le joueur installe le launcher une fois. Ensuite, à chaque ouverture, il :

- télécharge seulement les fichiers de mods qui ont changé (vérifiés par SHA-256) ;
- lance Arma 3 avec BattlEye, les bons mods et la connexion directe au serveur (mot de passe compris) ;
- affiche l'état du serveur (joueurs connectés, carte), vos actualités et vos liens (Discord, TeamSpeak, site) ;
- propose le plugin TeamSpeak TFAR s'il manque (facultatif) ;
- se met à jour tout seul quand vous publiez une nouvelle version.

Côté serveur, **aucun code ne tourne** : nginx sert des fichiers statiques, et un petit outil Python (dans Docker lui aussi) prépare ces fichiers à partir de votre dossier de mods.

## Principe

```
 Votre PC (Windows)                       Votre VPS (Docker)                        PC des joueurs
 ──────────────────                       ──────────────────                        ──────────────
 launcher.config.json                     serveur/
   nom, URL, couleurs, clé publique         mods/@CBA_A3 @MonMod …   (vos mods)
        │                                   config/launcher.json     (news, liens)
        ▼                                   launcher-build/          (installeur signé)
 npm run tauri build ──── copie ─────────►        │
   installeur .exe + .sig                         ▼  docker compose run --rm publier
                                            donnees/                                  Launcher
                                              launcher.json  manifest.json   ◄─HTTPS──  lit la config,
                                              mods/…  tfar/…  index.html                compare, télécharge
                                              update/latest.json + .exe                 le delta, lance Arma 3
                                                  ▲
                                            nginx (docker compose up -d)
```

## Démarrage rapide

1. **Le serveur.** Sur un VPS avec Docker et un nom de domaine qui pointe dessus :
   ```sh
   git clone <ce dépôt> && cd <dépôt>/serveur
   cp .env.exemple .env                                  # URL_PUBLIQUE, DOMAINE, EMAIL_ACME (garder HTTP_ECOUTE=127.0.0.1:8080)
   cp config/launcher.exemple.json config/launcher.json  # nom, adresse, port, liens, news
   docker compose --profile https up -d                  # nginx + HTTPS automatique (Caddy)
   ```
   Détails : [docs/01-installer-le-serveur.md](docs/01-installer-le-serveur.md).

2. **Les mods.** Copiez vos dossiers `@Mod` dans `serveur/mods/`, puis :
   ```sh
   docker compose run --rm publier --a-sec   # simulation
   docker compose run --rm publier           # publication
   ```

3. **Le launcher.** Sur un PC Windows avec Node.js 20+ et Rust : éditez `launcher.config.json` (nom, `urlServeur`, couleurs), remplacez les visuels de `launcher/public/` et `launcher/app-icon.png`.
   Voir [docs/02-personnaliser-le-launcher.md](docs/02-personnaliser-le-launcher.md).

4. **La clé de signature et l'installeur.** `npx tauri signer generate`, clé publique dans `launcher.config.json`, puis `npm run tauri build`.
   Voir [docs/03-signer-et-construire.md](docs/03-signer-et-construire.md).

5. **La publication.** Copiez l'installeur et son `.sig` dans `serveur/launcher-build/`, relancez `docker compose run --rm publier`. La page `https://votre-domaine/` propose alors le téléchargement, et les launchers déjà installés se mettent à jour.
   Voir [docs/04-publier-une-mise-a-jour.md](docs/04-publier-une-mise-a-jour.md).

## Documentation

| Fichier | Contenu |
|---|---|
| [01-installer-le-serveur.md](docs/01-installer-le-serveur.md) | VPS, Docker, DNS, HTTPS (profil Caddy, reverse proxy existant, certbot) |
| [02-personnaliser-le-launcher.md](docs/02-personnaliser-le-launcher.md) | `launcher.config.json`, logo, icônes, couleurs |
| [03-signer-et-construire.md](docs/03-signer-et-construire.md) | Clé de mise à jour, construction, signature manuelle |
| [04-publier-une-mise-a-jour.md](docs/04-publier-une-mise-a-jour.md) | Mods, news, nouvelle version du launcher |
| [05-format-des-fichiers.md](docs/05-format-des-fichiers.md) | `launcher.json`, `manifest.json`, `latest.json`, règles de validation |
| [06-depannage.md](docs/06-depannage.md) | Problèmes courants |

## Arborescence

```
launcher.config.json     LE fichier à éditer pour personnaliser le launcher
launcher/                application Tauri 2 (Rust + TypeScript/Vite)
  scripts/configurer.mjs   reporte launcher.config.json dans tauri.conf.json, package.json, Cargo.toml
  scripts/visuels.py       génère des visuels neutres (Pillow)
  src-tauri/               backend Rust (synchronisation, lancement, A2S, TFAR…)
  src/                     interface
serveur/                 serveur de mises à jour (Docker)
  docker-compose.yml       nginx, outil « publier », profil « https » (Caddy)
  nginx/                   configuration nginx
  publier/                 outil de publication (Python, sans dépendance)
  config/                  launcher.json (à créer depuis launcher.exemple.json)
docs/                    documentation
```

## Prérequis

- Serveur : Linux avec Docker Engine et le plugin Compose (`docker compose`).
- Construction du launcher : Windows 10/11, [Node.js](https://nodejs.org) 20 ou plus, [Rust](https://rustup.rs) stable (MSVC), les [prérequis Tauri](https://tauri.app/start/prerequisites/) (WebView2 est déjà présent sur Windows 10/11 à jour).
- Joueurs : Windows 10/11 64 bits, Arma 3 sur Steam.

## Licence

[MIT](LICENSE). Voir aussi [CONTRIBUTING.md](CONTRIBUTING.md).
