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
  "imageFond": "fond.webp",
  "couleurs": { "fond": "#0b0d14", "accent": "#ffb547" }
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
| `imageFond` | Image d'ambiance affichée plein cadre derrière l'écran « Jouer » (voir [Image de fond](#image-de-fond)). Vide : dégradé seul. | Interface |
| `couleurs` | Couleurs de l'interface (voir plus bas). | Interface, `scripts/visuels.py` |

### Comment la configuration est appliquée

- **Le code Rust** la lit à la compilation : `launcher/src-tauri/build.rs` en fait des variables `env!("LAUNCHER_URL_SERVEUR")`, `env!("LAUNCHER_NOM")`, `env!("LAUNCHER_DOSSIER")`, `env!("LAUNCHER_USER_AGENT")`. Toute modification du fichier relance la compilation concernée.
- **L'interface** la lit au moment de `vite build` (`launcher/vite.config.ts`) : nom, devise, couleurs et image de fond sont injectés dans `index.html`.
- **`tauri.conf.json`, `package.json`, `Cargo.toml`** ne savent pas lire un autre fichier : `npm run configurer` (`launcher/scripts/configurer.mjs`) y reporte nom, version, identifiant, URL de mise à jour et clé publique. Cette commande est lancée **automatiquement** par `npm run tauri dev` et `npm run tauri build` ; ne modifiez donc pas ces champs à la main, ils seraient écrasés.

`npm run configurer -- --verifier` n'écrit rien et échoue si un fichier n'est pas à jour (utile en intégration continue). Les deux commandes contrôlent aussi `imageFond` : fichier présent, format accepté, avertissement au-delà de 400 Ko.

## Visuels

| Fichier | Taille | Rôle |
|---|---|---|
| `launcher/public/logo.png` | 256 × 256, PNG transparent | Logo de la barre de titre (affiché en 24 px) |
| `launcher/app-icon.png` | 1024 × 1024, PNG transparent | Source de toutes les icônes (fenêtre, installeur, barre des tâches) |
| `launcher/public/fond.webp` | 1650 × 1020, WebP (95 Ko) | Image de fond par défaut (`imageFond`) |

Les visuels fournis sont volontairement neutres : un réticule dans la couleur `accent`, et une image d'ambiance (crépuscule sur la mer, île, brume) entièrement dessinée par calcul, sans aucune image protégée. Pour régénérer le logo et l'icône dans vos couleurs (nécessite Python et Pillow) :

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

`python launcher/scripts/visuels.py --fond` régénère l'image de fond par défaut (Pillow et numpy : `pip install pillow numpy`).

## Image de fond

L'écran « Jouer » pose le nom du serveur, l'état du serveur et le bouton **JOUER** sur une grande image plein cadre, sous un voile sombre (à gauche et en bas, pour la lisibilité). Les vignettes d'actualités et la première actualité de l'écran *Actualités* en reprennent un morceau ; les autres écrans la gardent floutée et assombrie.

```json
"imageFond": "fond.webp"
```

- **Où mettre le fichier** : soit dans `launcher/public/` (indiquez alors juste son nom, comme `fond.webp`), soit n'importe où dans le dépôt, avec un chemin relatif à la racine (`"imageFond": "visuels/mon-fond.jpg"`), copié à la construction sous `/fond-perso.jpg`. Formats : `.webp` (conseillé), `.jpg`, `.png`, `.avif`.
- **Quelle image** : une capture de votre serveur en jeu, sans texte ni interface, avec le sujet sur la **droite** (la gauche et le bas sont couverts par le texte et les vignettes). 1650 × 1020 suffit (la fenêtre fait 1100 × 680, agrandie selon l'écran) ; visez moins de 400 Ko en WebP. Utilisez une image dont vous avez les droits.
- **Sans image** : `"imageFond": ""` garde un dégradé teinté par `accent`.
- L'image est embarquée dans le launcher (aucune requête vers Internet) : la CSP reste `img-src 'self' data:`. La changer demande donc une nouvelle version du launcher.

La page de téléchargement générée par `publier` peut reprendre la même image : déposez-la sur le serveur dans `serveur/config/fond.webp` (ou `.jpg`, `.png`, `.avif`) ; `publier` la publie à côté de `index.html` et la place en tête de page. Sans ce fichier, la page garde un dégradé.

## Interface et couleurs

L'interface suit une direction « cinématique », celle des launchers de jeux récents : image d'ambiance plein cadre, nom du serveur en très grand (la seconde moitié en lettres évidées, taille ajustée automatiquement à la longueur du nom), relevés du serveur (joueurs, ping, carte, prochain redémarrage), liens, puis les trois dernières actualités en vignettes et le bouton **JOUER** lumineux en bas à droite. La barre de titre (déplaçable) porte les onglets *Jouer*, *Actualités* et *Liens*, les *Paramètres* (icône réglages), *Réduire* et *Fermer*. Hors de l'écran « Jouer », l'image passe floutée en arrière-plan et la synchronisation se replie en un bandeau compact, toujours visible.

![Écran Jouer](captures/accueil.png)

Chaque clé de `couleurs` devient une variable CSS `--<clé>` qui remplace celle de `launcher/src/styles.css`. En pratique, changer `accent` suffit à donner votre couleur au launcher :

| Clé | Rôle (valeur par défaut) |
|---|---|
| `fond` | Fond de la fenêtre et teinte du voile posé sur l'image (`#0b0d14`, nuit bleutée) |
| `surface`, `relief` | Dégradés de repli sans image, vignettes, options des listes (`#141824`, `#1e2330`) |
| `texte` | Texte principal ; les textes secondaires en sont dérivés (`#f4efe6`) |
| `accent` | **La** couleur de marque : bouton JOUER et sa lueur, jauge, onglet actif, dates, liens, focus (`#ffb547`, ambre) |
| `sur-accent` | Texte posé sur l'accent, bouton JOUER compris (`#1d1205`) |
| `alerte` | Bandeaux d'avertissement (TFAR, TeamSpeak), bouton « Configurer Arma 3 », lancement retenu pendant un redémarrage (`#ffd166`) |
| `ok`, `danger` | Serveur en ligne, erreurs (`#7be0a0`, `#ff7a6b`) |
| `trait`, `trait-fort` | Bordures (`rgb(255 255 255 / .12)`, `… / .22`) |

Les teintes dérivées (textes secondaires, verre des panneaux, voile de l'image, reflets du bouton JOUER) sont calculées à partir de `texte`, `fond`, `accent`, `alerte` et `danger` (`color-mix`).

**Contraste.** L'accent sert aussi de couleur de texte (dates, liens) sur `fond` et `surface` : gardez un rapport de contraste d'au moins 4,5:1 (WCAG AA). L'ambre par défaut atteint 11:1 sur `fond` et 10:1 sur `surface`, et `sur-accent` 10,5:1 sur l'accent. Un accent très clair (jaune, cyan pâle) demande un `sur-accent` sombre ; un accent très sombre, un `sur-accent` clair (`#ffffff`). Vérifiez avec un outil comme le *Contrast Checker* de WebAIM.

La page de téléchargement générée par `publier` (`https://votre-domaine/`) reprend la même identité (grand titre condensé, bouton ambre lumineux, image de fond si elle est publiée) ; ses couleurs sont dans le bloc `:root` du gabarit `PAGE` de `serveur/publier/publier.py`.

![Page de téléchargement](captures/page-telechargement.png)

## Aller plus loin

- **Polices** : Big Shoulders Display (titres, bouton JOUER, chiffres), Barlow (texte) et Barlow Condensed (étiquettes, onglets), toutes sous licence OFL, paquets `@fontsource/big-shoulders-display`, `@fontsource/barlow` et `@fontsource/barlow-condensed` importés en tête de `launcher/src/styles.css` (sous-ensemble latin seulement). Elles sont embarquées dans le launcher : aucune requête vers Internet, la CSP n'autorise que `font-src 'self'`. La page de téléchargement, servie sans police, se replie sur Bahnschrift (Windows).
- **Textes** : l'interface est en français, dans `launcher/index.html` et `launcher/src/main.ts`.
- **Taille de la fenêtre** : 1100 × 680, mise à l'échelle selon l'écran ; les constantes sont dans `src-tauri/src/lib.rs` (`LARGEUR`, `HAUTEUR`), `main.ts` (`echelle`) et `styles.css` (`body`).
- **Ping** : temps d'aller-retour de la requête A2S au port de requête du serveur de jeu (`port + 1`), mesuré par `src-tauri/src/a2s.rs` à chaque interrogation (toutes les 30 s).
- **Lien de téléchargement de TeamSpeak 3** : constante `TELECHARGER_TEAMSPEAK` de `main.ts`.

## Tester sans construire l'installeur

```sh
cd launcher
npm install
npm run tauri dev
```

La fenêtre s'ouvre en mode développement et interroge `urlServeur`. Pour essayer avec un serveur local, mettez temporairement `"urlServeur": "http://127.0.0.1:8080"` (le script `configurer` prévient que `http://` est réservé aux essais). Le launcher accepte alors le `http` pour toutes ses requêtes ; `configurer` active aussi `dangerousInsecureTransportProtocol` du plugin de mise à jour, sans quoi le launcher refuse de démarrer avec une URL en `http://`. Ce drapeau disparaît dès que `urlServeur` repasse en `https://`. Ne distribuez jamais un launcher construit en `http://`.

![Synchronisation des mods](captures/synchronisation.png)
