# 4. Publier une mise à jour

Toute publication passe par la même commande, à lancer dans `serveur/` sur le VPS :

```sh
docker compose run --rm publier --a-sec   # montre ce qui serait fait, n'écrit rien
docker compose run --rm publier           # publie
```

## Que faut-il renvoyer depuis le PC ?

| Ce qui change | Sur le PC | À envoyer sur le VPS | Sur le VPS |
|---|---|---|---|
| Mods (ajout, mise à jour, retrait) | rien | les dossiers `@Mod` dans `mods/` | `docker compose run --rm publier` |
| News, liens, adresse du serveur de jeu, TFAR | rien | rien (éditez `config/launcher.json` sur le VPS) | `docker compose run --rm publier` |
| Nouvelle version du launcher (code, couleurs, nom) | monter `version` dans `launcher.config.json`, `preparer.bat` | seulement le contenu de `a-envoyer/launcher-build/` (`.exe` et `.exe.sig`) dans `launcher-build/` | `docker compose run --rm publier --notes "…"` |
| Mise à jour de l'outil `publier`, de nginx ou de `docker-compose.yml` (nouvelle version de ce dépôt) | `preparer.bat` | `docker-compose.yml`, `nginx/`, `publier/`, `https/` (pas `.env`, `config/`, `mods/`, `donnees/`) | `docker compose up -d`, puis `publier` |

`preparer.bat` reprend `serveur/.env` et `serveur/config/launcher.json` s'ils existent sur le PC ; sinon il les crée depuis les exemples. Ne renvoyez pas ces deux fichiers par-dessus ceux du VPS si vous les y avez modifiés.

L'outil `publier` compare ce que vous avez déposé (`mods/`, `config/launcher.json`, `launcher-build/`) avec ce qui est en ligne (`donnees/`) et ne copie que ce qui a changé. Il regénère à chaque fois la page d'accueil `index.html`. Lancer la commande deux fois de suite ne fait rien la seconde fois.

## Mods : ajouter, mettre à jour, retirer

1. Déposez les dossiers `@Mod` dans `serveur/mods/` (WinSCP, `scp -r`, `rsync -av --delete`…). Les dossiers dont le nom ne commence pas par `@` sont ignorés, ainsi que, dans les mods, les fichiers et dossiers cachés (nom commençant par `.`, que nginx ne sert pas) et les fichiers `.part`. Copiez les fichiers eux-mêmes : `publier` refuse les liens symboliques (le conteneur ne voit pas leur cible).
2. Pour retirer un mod, supprimez son dossier de `serveur/mods/`.
3. `docker compose run --rm publier --a-sec`, vérifiez la liste, puis sans `--a-sec`.

**Rien d'autre à éditer** : la liste `mods` de `launcher.json` est dressée à partir des dossiers présents et injectée à la publication. Au prochain démarrage (ou au prochain clic sur *Mettre à jour*), le launcher des joueurs télécharge les fichiers nouveaux ou modifiés et supprime ceux qui ont disparu, **uniquement dans les dossiers des mods listés** : il ne touche jamais aux autres mods du joueur.

### Ordre de chargement

Par défaut, les mods sont passés à Arma (`-mod=`) dans l'ordre alphabétique. Pour imposer un ordre, ajoutez une liste `"mods"` dans `config/launcher.json` : les mods cités viennent en premier dans cet ordre, les autres suivent par ordre alphabétique.

```json
"mods": ["@CBA_A3", "@ace"]
```

### Plugin TeamSpeak TFAR (facultatif)

Si votre serveur utilise Task Force Radio, voir la section [TFAR](#tfar-task-force-radio) plus bas.

### Ordre de publication (pourquoi les joueurs connectés ne voient pas d'état cassé)

`publier` écrit toujours dans cet ordre :

1. les fichiers de mods nouveaux ou modifiés (chaque fichier est copié sous un nom temporaire puis renommé d'un coup) ;
2. `launcher.json`, si son contenu change (liste des mods comprise) ;
3. `manifest.json`, remplacé d'un coup ;
4. la suppression des fichiers disparus.

Le manifeste en ligne ne cite donc jamais un fichier absent. Le launcher relit `launcher.json` juste avant chaque `manifest.json` : un joueur dont le launcher est resté ouvert pendant l'ajout d'un mod reçoit la nouvelle liste, sans redémarrer. Un joueur qui télécharge pendant la publication peut au pire recevoir une empreinte différente de celle attendue : le launcher réessaie puis affiche une erreur, et un nouveau clic sur *Mettre à jour* suffit.

## News, liens, adresse du serveur de jeu

Éditez `serveur/config/launcher.json` (format : [05-format-des-fichiers.md](05-format-des-fichiers.md)) puis lancez `publier`. Aucune nouvelle version du launcher n'est nécessaire : il relit `launcher.json` à chaque ouverture.

Exemple de news :

```json
{ "date": "2026-02-01", "titre": "Nouvelle carte", "texte": "Le serveur passe sur Tanoa ce soir.\n• Redémarrage à 21 h", "lien": { "nom": "Voir l'annonce", "url": "https://discord.gg/exemple" } }
```

Les news sont triées par date (la plus récente en haut) et affichées en texte brut : pas de HTML, mais `\n` passe à la ligne.

## TFAR (Task Force Radio)

TFAR fait passer la voix et la radio du jeu par TeamSpeak : chaque joueur doit avoir, dans son TeamSpeak, le **plugin de la même version que le mod du serveur**. Le launcher peut le vérifier et l'installer, à condition que vous l'activiez dans `config/launcher.json` :

```json
"tfar": { "actif": true, "version": "stable", "plugin": "" }
```

| Champ | Rôle |
|---|---|
| `actif` | `false` (par défaut dans l'exemple) : rien de TFAR n'est publié, le launcher n'affiche ni bandeau, ni bouton, ni vérification. |
| `version` | `stable` ou `beta`, la version du mod TFAR de votre serveur (voir plus bas). Sert à l'affichage et à un contrôle : si le plugin trouvé est d'une autre version, `publier` vous prévient et publie la version réelle du plugin. |
| `plugin` | Vide : détection automatique, `publier` cherche le fichier `.ts3_plugin` dans les mods déposés (TFAR le livre dans `@…/teamspeak/`). Sinon, chemin relatif à `mods/` ou à `config/`, par exemple `@task_force_radio/teamspeak/task_force_radio.ts3_plugin` ou `tfar/mon-plugin.ts3_plugin` (fichier posé dans `config/tfar/`). |

Avec `actif: true`, `publier` copie le plugin dans `donnees/tfar/` et ajoute à `launcher.json` `tfarPlugin`, `tfarVersion` et `tfarDll` (empreinte SHA-256 de la DLL 64 bits contenue dans le plugin). Le launcher compare cette empreinte à la DLL installée dans `%APPDATA%\TS3Client\plugins\` et propose au joueur d'installer ou de mettre à jour le plugin (double-clic sur le `.ts3_plugin`, TeamSpeak fait le reste). Si plusieurs plugins **différents** se trouvent dans les mods, `publier` s'arrête et vous demande de préciser `plugin`.

### Stable ou bêta ?

| | Stable (0.9.x, en pratique 0.9.12) | Bêta (1.0) |
|---|---|---|
| Dossier du mod | `@task_force_radio` | `@TFAR` (GitHub) ; nom variable sur le Workshop |
| Plugin livré | `@task_force_radio/teamspeak/task_force_radio.ts3_plugin` | `@TFAR/teamspeak/task_force_radio.ts3_plugin` |
| DLL installée chez le joueur | `task_force_radio_win64.dll` | `TFAR_win64.dll` |

La plupart des serveurs tournent en stable 0.9.x ; certains utilisent la 1.0 bêta. Les deux ne se mélangent pas : un joueur qui a le plugin 0.9 n'entend rien sur un serveur 1.0, et inversement. Le launcher regarde la DLL qui correspond à la version publiée.

### Ce qu'il faut dire aux joueurs

- **TeamSpeak 3, en 64 bits.** TeamSpeak 5 et 6 ne chargent pas les plugins TFAR. Le launcher signale un TeamSpeak 3 absent et propose de le télécharger.
- **Pas de TFAR du Workshop en parallèle.** Un TFAR abonné sur le Steam Workshop (ou chargé par un autre launcher) dans une autre version peut remplacer le plugin dans TeamSpeak ou charger un second mod TFAR en jeu : désabonnez-le, le launcher fournit le bon.
- Fermer TeamSpeak avant d'installer ou de mettre à jour le plugin (TeamSpeak verrouille la DLL qu'il utilise).

Les anciennes configurations sans bloc `tfar` restent acceptées : un `.ts3_plugin` posé à la racine de `serveur/mods/` est encore publié (avec un avertissement), et un ancien `launcher.json` qui ne porte que `tfarPlugin` reste lu par le launcher.

## Nouvelle version du launcher

1. Sur le PC Windows : montez `"version"` dans `launcher.config.json`, puis double-cliquez sur `preparer.bat` (ou `npm run tauri build`, voir [03-signer-et-construire.md](03-signer-et-construire.md)).
2. Copiez **les deux fichiers** `…_x64-setup.exe` et `…_x64-setup.exe.sig` (dans `a-envoyer/launcher-build/`, ou `launcher/src-tauri/target/release/bundle/nsis/`) dans `launcher-build/` sur le VPS :
   ```powershell
   scp "launcher\src-tauri\target\release\bundle\nsis\Arma Launcher_0.1.1_x64-setup.exe*" user@vps:arma-launcher/serveur/launcher-build/
   ```
3. Sur le VPS :
   ```sh
   docker compose run --rm publier --notes "Corrections diverses"
   ```

`publier` prend dans `launcher-build/` l'installeur de **plus haute version**, le copie dans `donnees/update/` sous deux noms (`<nom>_<version>_x64-setup.exe` pour la mise à jour automatique, `launcher-setup.exe` pour le bouton de la page d'accueil), supprime l'installeur de la version précédente et écrit `update/latest.json`. `URL_PUBLIQUE` (fichier `.env`) doit être renseignée : l'adresse de l'installeur y est absolue.

Au prochain démarrage, les launchers installés proposent « Mise à jour du launcher : la version X est disponible ».

Les anciens installeurs peuvent rester dans `launcher-build/` : seul le plus récent est publié.

**Installeur sans `.sig`** (construit sans clé) : `publier` le met quand même en téléchargement sur la page d'accueil, mais n'écrit pas `update/latest.json` (et retire celui qui existait) : aucun launcher installé ne se met à jour, les joueurs doivent retélécharger l'installeur.

## Automatiser depuis votre PC (exemple)

Un script PowerShell de quelques lignes suffit pour tout enchaîner :

```powershell
$vps = "user@play.example.com"
scp -r .\mods\* "${vps}:arma-launcher/serveur/mods/"
ssh $vps "cd arma-launcher/serveur && docker compose run --rm publier"
```

Pour de gros mods, préférez `rsync` (via WSL) qui ne renvoie que les fichiers modifiés.
