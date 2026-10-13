# 5. Format des fichiers

Toutes les adresses ci-dessous sont relatives à `urlServeur` (`launcher.config.json`), par exemple `https://play.example.com`.

| URL | Fichier dans `serveur/donnees/` | Produit par | Cache |
|---|---|---|---|
| `/` | `index.html` | `publier` | `no-cache` |
| `/launcher.json` | `launcher.json` | `publier` (depuis `config/launcher.json`) | `no-cache` |
| `/manifest.json` | `manifest.json` | `publier` | `no-cache` |
| `/mods/<chemin>` | `mods/<chemin>` | `publier` (copie de `serveur/mods/`) | 1 jour, 5 min pour un CDN |
| `/tfar/<plugin>.ts3_plugin` | `tfar/…` | `publier` (facultatif) | `no-cache` |
| `/update/latest.json` | `update/latest.json` | `publier` (depuis `launcher-build/`) | `no-cache` |
| `/update/<nom>_<version>_x64-setup.exe` | idem | `publier` | `no-cache` |
| `/update/launcher-setup.exe` | idem (copie) | `publier` | `no-cache` |
| `/<etatServeur>` | à vous de le fournir | facultatif | à votre choix |

## `launcher.json`

Lu par le launcher à chaque démarrage. Vous éditez `serveur/config/launcher.json` ; `publier` y ajoute `mods` et, si le bloc `tfar` est actif, `tfarPlugin`, `tfarVersion` et `tfarDll` avant de le publier (le bloc `tfar` lui-même n'est pas publié).

```json
{
  "version": 1,
  "serveur": { "nom": "Mon Serveur Arma 3", "hote": "play.example.com", "port": 2302, "motDePasse": "" },
  "mods": ["@CBA_A3", "@mon_mod"],
  "parametres": "-skipIntro -noSplash",
  "liens": [
    { "nom": "TeamSpeak", "url": "ts3server://play.example.com?port=9987", "icone": "teamspeak" },
    { "nom": "Discord", "url": "https://discord.gg/exemple", "icone": "discord" },
    { "nom": "Site", "url": "https://www.example.com", "icone": "site", "description": "Règlement et actualités" }
  ],
  "news": [
    { "date": "2026-01-15", "titre": "Bienvenue", "texte": "Texte brut.\nDeuxième ligne.", "lien": { "nom": "En savoir plus", "url": "https://www.example.com" } }
  ],
  "tfarPlugin": "tfar/task_force_radio.ts3_plugin",
  "tfarVersion": "stable",
  "tfarDll": "<sha256 de la DLL 64 bits du plugin>",
  "etatServeur": "api/serveur/etat"
}
```

| Champ | Obligatoire | Rôle |
|---|---|---|
| `version` | oui | Version du **format**, toujours `1`. Un launcher qui ne la connaît pas affiche « mettez à jour le launcher » et ne lance pas le jeu. |
| `serveur.nom` | oui | Nom affiché en titre de l'écran « Jouer » et sur la page de téléchargement. |
| `serveur.hote` | oui | Adresse du serveur de jeu (nom ou IP) : `-connect=`. |
| `serveur.port` | oui | Port de jeu (`-port=`, souvent `2302`). L'état du serveur est demandé en A2S (requête Steam) sur **port + 1**. |
| `serveur.motDePasse` | non | Mot de passe du serveur (`-password=`). **Public** : `launcher.json` est lisible par tout le monde. À réserver à un mot de passe sans valeur secrète, ou à laisser vide. |
| `mods` | ajouté | Liste des dossiers `@Mod`, dans l'ordre de `-mod=`. Ce sont les **seuls** dossiers que le launcher gère (télécharge, met à jour, nettoie). Dans `config/launcher.json`, une liste `mods` facultative ne sert qu'à imposer l'ordre (voir [04](04-publier-une-mise-a-jour.md#ordre-de-chargement)). |
| `parametres` | non | Paramètres Arma ajoutés à la ligne de commande, séparés par des espaces. Le launcher ajoute `-world=empty` (démarrage plus rapide) sauf si un `-world=` est déjà fourni ici ou par le joueur. |
| `liens[]` | non | Boutons de l'écran « Liens » et du bloc « Rejoindre » de l'écran « Jouer ». `nom`, `url`, `icone` (`teamspeak`, `discord`, `site` ; toute autre valeur prend l'icône `site`), `description` facultative (sous-titre). |
| `news[]` | non | Actualités : `date` (`AAAA-MM-JJ`), `titre`, `texte` (texte brut, `\n` pour aller à la ligne), `lien` facultatif (`nom`, `url`). La plus récente de moins de 7 jours porte un badge « Nouveau ». |
| `tfar` | non | Dans `config/launcher.json` seulement : `{ "actif": false, "version": "stable", "plugin": "" }`. Voir [04, TFAR](04-publier-une-mise-a-jour.md#tfar-task-force-radio). `actif: false` ou bloc absent sans `.ts3_plugin` à la racine de `mods/` : aucun des trois champs suivants n'est publié et le launcher n'affiche rien de TFAR. |
| `tfarPlugin` | ajouté | Chemin relatif du plugin TeamSpeak TFAR. Absent : aucune vérification du plugin. |
| `tfarVersion` | ajouté | `stable` (TFAR 0.9.x) ou `beta` (TFAR 1.0), lue dans le plugin. Choisit la DLL que le launcher contrôle : `task_force_radio_win64.dll` (stable) ou `TFAR_win64.dll` (bêta). Absente (ancien `launcher.json`) : l'une ou l'autre. |
| `tfarDll` | ajouté | SHA-256 (64 caractères hexadécimaux) de la DLL 64 bits contenue dans le plugin (`plugins/task_force_radio_win64.dll` ou `plugins/TFAR_win64.dll`). Le launcher la compare à la DLL installée dans `%APPDATA%\TS3Client\plugins\` pour signaler un plugin périmé. |
| `etatServeur` | non | Chemin relatif d'un point d'API **que vous fournissez** (voir plus bas). Absent : le launcher ne s'appuie que sur la requête A2S. |

### Règles de validation du launcher

Le launcher refuse le fichier entier (message d'erreur, pas de lancement) si :

- `version` n'est pas `1` ;
- `mods` est vide, ou un nom ne commence pas par `@`, contient `/`, `\`, `:`, ou vaut `.`/`..` ;
- `tfarPlugin`, ou `etatServeur` (hors `?…`), n'est pas un chemin relatif sûr (voir plus bas) ;
- `tfarDll` n'est pas vide et n'est pas un SHA-256 hexadécimal.

Les **liens** dont l'URL ne commence pas par `https://`, `ts3server://` ou `discord://` sont retirés silencieusement (pas de `http://`, `file://`, `javascript:`…). `publier` vous prévient s'il en trouve.

Les **liens TeamSpeak** (`ts3server://`) ont un rôle de plus : si l'option « Rejoindre le TeamSpeak au lancement » est cochée, le launcher ouvre le premier d'entre eux en lançant le jeu. Sans lien TeamSpeak ni plugin TFAR, les bandeaux et l'option TeamSpeak sont masqués.

### `etatServeur` (facultatif)

Si vous avez un site ou une API capable de dire si le serveur tourne et quand il redémarre, exposez sous `urlServeur` (par votre reverse proxy) un JSON de cette forme :

```json
{ "enLigne": true, "rebootDans": 5400 }
```

- `enLigne` : le serveur de jeu tourne.
- `rebootDans` : secondes avant le prochain redémarrage planifié (négatif juste après l'heure prévue), facultatif.

Le launcher affiche alors l'heure du prochain redémarrage et retient le bouton JOUER pendant les deux minutes qui précèdent un redémarrage et la minute qui suit (« Lancer au retour »). Un serveur muet en A2S n'est tenu pour éteint que si `enLigne` vaut `false` : un pare-feu qui coupe l'UDP du joueur ne l'empêche pas de jouer. Si le point d'API ne répond pas (404, délai), il est ignoré.

## `manifest.json`

Généré par `publier`, jamais édité à la main.

```json
{
 "genere": "2026-01-15T18:30:00Z",
 "fichiers": [
  { "chemin": "@CBA_A3/addons/cba_main.pbo", "taille": 300000, "sha256": "9f2c…64 caractères…" }
 ]
}
```

- `chemin` : relatif au dossier des mods, séparateur `/`, premier segment = un mod de `launcher.json > mods`.
- URL de téléchargement : `urlServeur + "/mods/" + chemin`, **chaque segment encodé en pourcentage** (tout sauf lettres, chiffres et `- _ . ~ @`). Exemple : `@CBA_A3/addons/cba main.pbo` → `/mods/@CBA_A3/addons/cba%20main.pbo`.
- Le launcher compare chaque fichier local (taille, puis SHA-256 avec un cache taille + date de modification) ; il télécharge ce qui manque ou diffère (4 en parallèle, 3 essais, fichier `.part` puis renommage après vérification de la taille et du SHA-256) et supprime, dans les mods gérés seulement, les fichiers absents du manifeste. La comparaison des noms ignore la casse (Windows).

### Chemins acceptés

Un `chemin` est accepté par le launcher **et** par `publier` seulement s'il :

- n'est pas vide, ne commence pas par `/` ;
- ne contient ni `\` ni `:` ;
- a au moins deux segments séparés par `/`, aucun vide, aucun égal à `.` ou `..` ;
- commence par un mod de la liste `mods`.

`publier` refuse en plus les liens symboliques, les noms invalides sous Windows (`< > " | ? *`, caractères de contrôle, segment finissant par un point ou une espace) et deux chemins qui ne diffèrent que par la casse ; il ignore les fichiers et dossiers cachés et les fichiers `.part`.

Un seul chemin refusé fait rejeter **tout** le manifeste (« Manifeste refusé, chemin invalide »). Un SHA-256 qui n'a pas 64 caractères hexadécimaux aussi.

## `update/latest.json`

Format de l'updater Tauri 2, écrit par `publier` :

```json
{
 "version": "0.1.1",
 "notes": "Corrections diverses",
 "pub_date": "2026-01-15T18:30:00Z",
 "platforms": {
  "windows-x86_64": {
   "signature": "<contenu du fichier .sig>",
   "url": "https://play.example.com/update/Arma-Launcher_0.1.1_x64-setup.exe"
  }
 }
}
```

- `version` : comparée à celle du launcher installé ; mise à jour proposée si elle est supérieure.
- `signature` : contenu du `.sig` produit à la construction, vérifié avec `clePubliqueMaj` compilée dans le launcher.
- `url` : adresse **absolue** de l'installeur, d'où le besoin de `URL_PUBLIQUE` dans `.env`. Les espaces du nom de produit sont remplacés par `-` dans le nom publié.

## Dossiers côté joueur

| Dossier | Contenu |
|---|---|
| `%APPDATA%\<dossierDonnees>\config.json` | Réglages (dossier Arma 3, dossier des mods, paramètres, comportement après lancement) |
| `%APPDATA%\<dossierDonnees>\launcher.json` | Dernier `launcher.json` reçu, utilisé hors ligne |
| `%APPDATA%\<dossierDonnees>\hash-cache.json` | Cache des empreintes |
| `%LOCALAPPDATA%\<dossierDonnees>\mods\` | Mods (par défaut ; modifiable dans les paramètres, les mods sont alors déplacés) |

Hors ligne, le launcher reprend le `launcher.json` en cache et laisse jouer si tous les dossiers de mods existent.
