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
  "couleurs": { "fond": "#141618", "accent": "#8a9a5b" }
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
| `devise` | Phrase affichée sous le nom du serveur, sur l'écran « Jouer ». | Interface |
| `couleurs` | Couleurs de l'interface (voir plus bas). | Interface, `scripts/visuels.py` |

### Comment la configuration est appliquée

- **Le code Rust** la lit à la compilation : `launcher/src-tauri/build.rs` en fait des variables `env!("LAUNCHER_URL_SERVEUR")`, `env!("LAUNCHER_NOM")`, `env!("LAUNCHER_DOSSIER")`, `env!("LAUNCHER_USER_AGENT")`. Toute modification du fichier relance la compilation concernée.
- **L'interface** la lit au moment de `vite build` (`launcher/vite.config.ts`) : nom, devise et couleurs sont injectés dans `index.html`.
- **`tauri.conf.json`, `package.json`, `Cargo.toml`** ne savent pas lire un autre fichier : `npm run configurer` (`launcher/scripts/configurer.mjs`) y reporte nom, version, identifiant, URL de mise à jour et clé publique. Cette commande est lancée **automatiquement** par `npm run tauri dev` et `npm run tauri build` ; ne modifiez donc pas ces champs à la main, ils seraient écrasés.

`npm run configurer -- --verifier` n'écrit rien et échoue si un fichier n'est pas à jour (utile en intégration continue).

## Visuels

| Fichier | Taille | Rôle |
|---|---|---|
| `launcher/public/logo.png` | 256 × 256, PNG transparent | Logo de la barre latérale (affiché en 32 px) |
| `launcher/app-icon.png` | 1024 × 1024, PNG transparent | Source de toutes les icônes (fenêtre, installeur, barre des tâches) |

Les visuels fournis sont volontairement neutres (un réticule dans la couleur `accent`). Pour les régénérer dans vos couleurs (nécessite Python et Pillow) :

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

## Interface et couleurs

L'interface suit un style « tactique » sobre : fond graphite, **une seule couleur d'accent**, police à chasse fixe pour les données (état du serveur, joueurs, tailles, progression) et sans-serif pour le reste. Une barre latérale mène aux écrans *Jouer*, *Actualités*, *Liens* et *Paramètres* ; la zone du bas (état des mods, jauge de synchronisation, bouton **JOUER**) reste visible sur tous les écrans.

![Écran Jouer](captures/accueil.png)

Chaque clé de `couleurs` devient une variable CSS `--<clé>` qui remplace celle de `launcher/src/styles.css`. En pratique, changer `accent` suffit à donner votre couleur au launcher :

| Clé | Rôle (valeur par défaut) |
|---|---|
| `fond` | Fond de la fenêtre, graphite (`#141618`) |
| `surface`, `relief` | Barre latérale, cartes, champs et boutons secondaires (`#1c1f22`, `#262a2e`) |
| `texte`, `discret` | Texte principal et secondaire (`#e4e2da`, `#969a91`) |
| `accent` | **La** couleur de marque : bouton JOUER, jauge, élément actif, liens, focus (`#8a9a5b`, vert olive) |
| `sur-accent` | Texte posé sur l'accent, bouton JOUER compris (`#101210`) |
| `alerte` | Bandeaux d'avertissement, bouton « Configurer Arma 3 », lancement retenu pendant un redémarrage (`#d4a849`) |
| `ok`, `danger` | Serveur en ligne, erreurs (`#8fbf6a`, `#e0705f`) |
| `trait`, `trait-fort` | Bordures (`rgb(255 255 255 / .07)`, `… / .14`) |

Les teintes dérivées (fonds des bandeaux, sélection) sont calculées à partir de `accent`, `alerte` et `danger` (`color-mix`).

**Contraste.** L'accent sert aussi de couleur de texte (dates, liens) sur `fond` et `surface` : gardez un rapport de contraste d'au moins 4,5:1 (WCAG AA). Le vert olive par défaut atteint 5,9:1 sur `fond` et 5,4:1 sur `surface`, et `sur-accent` 6,1:1 sur l'accent. Un accent très clair (jaune, cyan pâle) demande un `sur-accent` sombre ; un accent très sombre, un `sur-accent` clair (`#ffffff`). Vérifiez avec un outil comme le *Contrast Checker* de WebAIM.

La page de téléchargement générée par `publier` (`https://votre-domaine/`) reprend la même identité ; ses couleurs sont dans le bloc `:root` du gabarit `PAGE` de `serveur/publier/publier.py`.

## Aller plus loin

- **Polices** : IBM Plex Sans et IBM Plex Mono (licence OFL), paquets `@fontsource/ibm-plex-sans` et `@fontsource/ibm-plex-mono` importés en tête de `launcher/src/styles.css` (sous-ensemble latin seulement). Elles sont embarquées dans le launcher : aucune requête vers Internet, la CSP n'autorise que `font-src 'self'`.
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
