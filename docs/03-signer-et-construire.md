# 3. Signer et construire le launcher

La mise à jour automatique (plugin updater de Tauri) n'installe une nouvelle version que si elle est **signée** par votre clé privée : un pirate qui prendrait la main sur votre serveur web ne pourrait pas pousser un faux launcher à vos joueurs. Il vous faut donc une paire de clés, créée une fois pour toutes.

## 1. Préparer la machine (Windows)

- [Node.js](https://nodejs.org) 20 ou plus.
- [Rust](https://rustup.rs) stable, chaîne MSVC (proposée par défaut par `rustup` sous Windows), avec les « Build Tools for Visual Studio » (charge de travail C++).
- WebView2 : déjà présent sur Windows 10/11 à jour.

Puis, une fois :

```sh
cd launcher
npm install
```

## 2. Générer la clé de mise à jour

```powershell
cd launcher
npx tauri signer generate -w "$env:USERPROFILE\.tauri\mon-launcher.key"
```

La commande demande un mot de passe (vous pouvez le laisser vide, voir plus bas) et écrit :

- `mon-launcher.key` : la **clé privée**. Secrète.
- `mon-launcher.key.pub` : la **clé publique**. Copiez son contenu (une longue ligne en base64) dans `"clePubliqueMaj"` de `launcher.config.json`.

> **Ne commitez jamais la clé privée.** Le `.gitignore` du dépôt ignore `*.key`, mais rangez-la de toute façon **hors** du dépôt (`%USERPROFILE%\.tauri\` par exemple) et **sauvegardez-la** ailleurs (gestionnaire de mots de passe, clé USB). Si vous la perdez, les launchers déjà installés ne pourront plus jamais se mettre à jour : chaque joueur devra réinstaller à la main un launcher signé par une nouvelle clé.

## 3. Construire l'installeur

PowerShell, depuis `launcher/` :

```powershell
$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content -Raw "$env:USERPROFILE\.tauri\mon-launcher.key"
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = "votre mot de passe"   # "" si la clé n'en a pas
npm run tauri build
```

`npm run tauri build` commence par `npm run configurer` (report de `launcher.config.json`), construit l'interface, compile le Rust en release puis produit, dans `launcher/src-tauri/target/release/bundle/nsis/` :

- `<nom>_<version>_x64-setup.exe` : l'installeur (installation par utilisateur, sans droits administrateur) ;
- `<nom>_<version>_x64-setup.exe.sig` : sa signature, que le serveur publie dans `update/latest.json`.

La première compilation prend quelques minutes, les suivantes beaucoup moins.

Si `clePubliqueMaj` est vide, la construction ne signe rien et ne produit que le `.exe` : pratique pour un essai, mais ce launcher ne se mettra jamais à jour tout seul.

### Piège : construction bloquée sur le mot de passe

Il arrive que `npm run tauri build` reste bloqué indéfiniment sur :

```
Info Decrypting updater signing key, expect a prompt for password
```

Constaté dans un terminal non interactif (tâche lancée par un outil, entrée standard fermée), quand la clé n'a **pas** de mot de passe : le CLI attend une saisie qui n'arrive jamais. À ce stade, **l'installeur est déjà écrit**, seule la signature manque. Interrompez (Ctrl+C, ou tuez le processus Node du CLI) puis signez à la main :

```powershell
npx tauri signer sign -f "$env:USERPROFILE\.tauri\mon-launcher.key" -p "" "src-tauri\target\release\bundle\nsis\Arma Launcher_0.1.0_x64-setup.exe"
```

(adaptez le nom de fichier et `-p` à votre mot de passe). La commande affiche `Your file was signed successfully` et écrit le `.sig` juste à côté de l'installeur.

Pour éviter le problème : donnez un mot de passe à la clé et passez-le dans `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, ou construisez dans un PowerShell interactif ordinaire.

## 4. Changer de version

1. Montez `"version"` dans `launcher.config.json` (`0.1.0` → `0.1.1`). C'est le seul endroit : `configurer` reporte la version dans `tauri.conf.json`, `package.json`, `package-lock.json`, `Cargo.toml` et `Cargo.lock`.
2. `npm run tauri build` comme ci-dessus.
3. Publiez : [04-publier-une-mise-a-jour.md](04-publier-une-mise-a-jour.md).

Un launcher installé ne propose une mise à jour que vers une version **strictement supérieure**. Pour revenir en arrière après une version ratée, publiez l'ancien code sous un numéro plus grand.

## 5. Construire sur GitHub (facultatif)

Le dépôt contient `.github/workflows/build.yml` :

- à chaque pull request : `npm ci`, `configurer --verifier`, construction de l'interface et `cargo test --lib` ;
- sur un tag `v*` ou à la demande (onglet *Actions* > *Construire le launcher* > *Run workflow*) : construction de l'installeur signé, déposé comme artefact `installeur-windows`.

Ajoutez dans *Settings* > *Secrets and variables* > *Actions* :

| Secret | Valeur |
|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` | Le **contenu** du fichier `.key` |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Son mot de passe (secret vide si la clé n'en a pas ; un mot de passe évite le piège ci-dessus) |

## Signature Windows (SmartScreen)

La clé ci-dessus ne signe que les **mises à jour**. Elle ne remplace pas une signature de code Windows (Authenticode) : sans certificat de signature de code, Windows SmartScreen affiche « Windows a protégé votre ordinateur » au premier lancement de l'installeur (le joueur clique sur *Informations complémentaires* puis *Exécuter quand même*). Pour l'éviter, il faut acheter un certificat de signature de code et le déclarer dans `bundle.windows` de `tauri.conf.json` (voir la documentation Tauri, *Windows Code Signing*). Ce n'est pas nécessaire au fonctionnement.
