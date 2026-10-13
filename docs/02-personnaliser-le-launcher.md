# 2. Personnaliser le launcher

Tout ce qui distingue **votre** launcher tient dans un seul fichier, à la racine du dépôt : `launcher.config.json`. Le contenu qui change souvent (news, liens, adresse du serveur de jeu) n'est pas là : il est publié par le serveur dans `launcher.json` et ne demande aucune nouvelle version du launcher (voir [04-publier-une-mise-a-jour.md](04-publier-une-mise-a-jour.md)).

## `launcher.config.json`

```json
{
  "nom": "Arma Launcher",
  "version": "0.1.0",
  "identifiant": "com.example.armalauncher",
  "dossierDonnees": "Arma Launcher",
  "urlServeur": "https://play.example.com",
  "clePubliqueMaj": "",
  "userAgent": "Arma-Launcher",
  "editeur": "",
  "devise": "Un serveur, mille histoires…",
  "couleurs": { "fond": "#0b0f12", "accent": "#6cb8e6" }
}
```

| Clé | Rôle | Où elle sert |
|---|---|---|
| `nom` | Nom du produit : titre de la fenêtre, info-bulle de la zone de notification, nom de l'installeur (`<nom>_<version>_x64-setup.exe`), menu Démarrer. | `tauri.conf.json`, Rust, interface |
| `version` | Version du launcher, `x.y.z`. La mise à jour automatique ne propose qu'une version **supérieure** à celle installée. | `tauri.conf.json`, `package.json`, `Cargo.toml` |
| `identifiant` | Identifiant unique de l'application, en notation inversée (`com.monserveur.launcher`). **À fixer une fois pour toutes** : le changer après distribution crée une autre application, installée à côté de l'ancienne. | `tauri.conf.json` |
| `dossierDonnees` | Nom du dossier dans `%APPDATA%` (réglages, cache) et `%LOCALAPPDATA%` (mods par défaut : `%LOCALAPPDATA%\<dossierDonnees>\mods`). Le changer après distribution fait perdre aux joueurs leurs réglages et retélécharger les mods. | Rust (`config.rs`) |
| `urlServeur` | Adresse du serveur de mises à jour, **sans `/` final**. Le launcher y lit `launcher.json`, `manifest.json`, `mods/…`, et l'updater `update/latest.json`. | Rust (`distant.rs`), `tauri.conf.json` |
| `clePubliqueMaj` | Clé publique de signature des mises à jour (voir [03-signer-et-construire.md](03-signer-et-construire.md)). Vide = mise à jour automatique désactivée, construction sans signature. | `tauri.conf.json` |
| `userAgent` | Préfixe de l'en-tête `User-Agent` des requêtes HTTP (`Arma-Launcher/0.1.0`), pratique pour filtrer vos journaux nginx. | Rust (`lib.rs`) |
| `editeur` | Facultatif : nom de l'éditeur affiché par l'installeur Windows. | `tauri.conf.json` |
| `devise` | Phrase affichée sous le nom de la carte, dans l'en-tête. | Interface |
| `couleurs` | Couleurs de l'interface (voir plus bas). | Interface, `scripts/visuels.py` |

### Comment la configuration est appliquée

- **Le code Rust** la lit à la compilation : `launcher/src-tauri/build.rs` en fait des variables `env!("LAUNCHER_URL_SERVEUR")`, `env!("LAUNCHER_NOM")`, `env!("LAUNCHER_DOSSIER")`, `env!("LAUNCHER_USER_AGENT")`. Toute modification du fichier relance la compilation concernée.
- **L'interface** la lit au moment de `vite build` (`launcher/vite.config.ts`) : nom, devise et couleurs sont injectés dans `index.html`.
- **`tauri.conf.json`, `package.json`, `Cargo.toml`** ne savent pas lire un autre fichier : `npm run configurer` (`launcher/scripts/configurer.mjs`) y reporte nom, version, identifiant, URL de mise à jour et clé publique. Cette commande est lancée **automatiquement** par `npm run tauri dev` et `npm run tauri build` ; ne modifiez donc pas ces champs à la main, ils seraient écrasés.

`npm run configurer -- --verifier` n'écrit rien et échoue si un fichier n'est pas à jour (utile en intégration continue).

## Visuels

| Fichier | Taille | Rôle |
|---|---|---|
| `launcher/public/logo.png` | 256 × 256, PNG transparent | Petit logo de la barre de titre |
| `launcher/public/entete.webp` | 1536 × 287 (ou plus, même proportion) | Fond de l'en-tête (état du serveur) |
| `launcher/app-icon.png` | 1024 × 1024, PNG transparent | Source de toutes les icônes (fenêtre, installeur, barre des tâches) |

Les visuels fournis sont volontairement neutres. Pour les régénérer dans vos couleurs (nécessite Python et Pillow) :

```sh
pip install pillow
python launcher/scripts/visuels.py
```

Après avoir remplacé `app-icon.png`, **régénérez les icônes** :

```sh
cd launcher
npx tauri icon app-icon.png
```

La commande écrit `src-tauri/icons/` (dont `icon.ico` utilisé par Windows). Elle crée aussi `icons/android/` et `icons/ios/`, inutiles ici et ignorés par git.

## Couleurs

Chaque clé de `couleurs` devient une variable CSS `--<clé>` qui remplace celle de `launcher/src/styles.css` :

| Clé | Rôle (valeur par défaut) |
|---|---|
| `fond` | Fond de la fenêtre (`#0b0f12`) |
| `surface`, `relief` | Panneaux et éléments surélevés (`#12181d`, `#1a2229`) |
| `texte`, `discret` | Texte principal et secondaire (`#eceeea`, `#8b98a1`) |
| `accent` | Couleur de marque : boutons, liens, jauge, focus (`#6cb8e6`) |
| `soleil` | Avertissements, bouton « Configurer Arma 3 » (`#e9b44c`) |
| `ok`, `danger` | Serveur en ligne, erreurs (`#5fc98a`, `#ef6a5e`) |
| `trait`, `trait-fort` | Bordures (`rgb(255 255 255 / .08)`, `… / .14`) |

Les teintes transparentes (survols, halos) sont calculées à partir de `accent` et `soleil` (`color-mix`), il n'y a rien d'autre à changer.

## Aller plus loin

- **Polices** : `@fontsource-variable/*`, importées en tête de `launcher/src/styles.css`. Elles sont embarquées dans le launcher (aucune requête vers Internet).
- **Décor de l'en-tête** : courbes de niveau dessinées par `launcher/src/topo.ts` (constante `COORDONNEE` pour la petite coordonnée affichée). Supprimez l'appel `poserTopo` dans `main.ts` pour vous en passer.
- **Textes** : l'interface est en français, dans `launcher/index.html` et `launcher/src/main.ts`.
- **Taille de la fenêtre** : 1100 × 680, mise à l'échelle selon l'écran ; les constantes sont dans `src-tauri/src/lib.rs` (`LARGEUR`, `HAUTEUR`), `main.ts` (`echelle`) et `styles.css` (`body`).
- **Lien de téléchargement de TeamSpeak 3** : constante `TELECHARGER_TEAMSPEAK` de `main.ts`.

## Tester sans construire l'installeur

```sh
cd launcher
npm install
npm run tauri dev
```

La fenêtre s'ouvre en mode développement et interroge `urlServeur`. Pour essayer avec un serveur local, mettez temporairement `"urlServeur": "http://127.0.0.1:8080"` (le script `configurer` prévient que `http://` est réservé aux essais).
