# 4. Publier une mise à jour

Toute publication passe par la même commande, à lancer dans `serveur/` sur le VPS :

```sh
docker compose run --rm publier --a-sec   # montre ce qui serait fait, n'écrit rien
docker compose run --rm publier           # publie
```

L'outil `publier` compare ce que vous avez déposé (`mods/`, `config/launcher.json`, `launcher-build/`) avec ce qui est en ligne (`donnees/`) et ne copie que ce qui a changé. Il regénère à chaque fois la page d'accueil `index.html`. Lancer la commande deux fois de suite ne fait rien la seconde fois.

## Mods : ajouter, mettre à jour, retirer

1. Déposez les dossiers `@Mod` dans `serveur/mods/` (WinSCP, `scp -r`, `rsync -av --delete`…). Les dossiers dont le nom ne commence pas par `@` sont ignorés.
2. Pour retirer un mod, supprimez son dossier de `serveur/mods/`.
3. `docker compose run --rm publier --a-sec`, vérifiez la liste, puis sans `--a-sec`.

**Rien d'autre à éditer** : la liste `mods` de `launcher.json` est dressée à partir des dossiers présents et injectée à la publication. Au prochain démarrage (ou au prochain clic sur *Mettre à jour*), le launcher des joueurs télécharge les fichiers nouveaux ou modifiés et supprime ceux qui ont disparu, **uniquement dans les dossiers des mods listés** : il ne touche jamais aux autres mods du joueur.

### Ordre de chargement

Par défaut, les mods sont passés à Arma (`-mod=`) dans l'ordre alphabétique. Pour imposer un ordre, ajoutez une liste `"mods"` dans `config/launcher.json` : les mods cités viennent en premier dans cet ordre, les autres suivent par ordre alphabétique.

```json
"mods": ["@CBA_A3", "@ace"]
```

### Plugin TeamSpeak TFAR (facultatif)

Si votre serveur utilise Task Force Radio, déposez le fichier `task_force_radio.ts3_plugin` **à la racine** de `serveur/mods/` (à côté des dossiers `@Mod`). `publier` le copie dans `donnees/tfar/`, ajoute `tfarPlugin` à `launcher.json` et, si l'archive contient `plugins/TFAR_win64.dll`, son empreinte `tfarDll` : le launcher signale alors au joueur un plugin absent ou différent de celui du serveur et propose de l'installer. Sans fichier `.ts3_plugin`, rien de tout cela n'apparaît.

### Ordre de publication (pourquoi les joueurs connectés ne voient pas d'état cassé)

`publier` écrit toujours dans cet ordre :

1. les fichiers de mods nouveaux ou modifiés (chaque fichier est copié sous un nom temporaire puis renommé d'un coup) ;
2. `launcher.json`, si son contenu change (liste des mods comprise) ;
3. `manifest.json`, remplacé d'un coup ;
4. la suppression des fichiers disparus.

Le manifeste en ligne ne cite donc jamais un fichier absent, ni un mod hors de la liste de `launcher.json` (le launcher refuserait le manifeste entier). Un joueur qui télécharge pendant la publication peut au pire recevoir une empreinte différente de celle attendue : le launcher réessaie puis affiche une erreur, et un nouveau clic sur *Mettre à jour* suffit.

## News, liens, adresse du serveur de jeu

Éditez `serveur/config/launcher.json` (format : [05-format-des-fichiers.md](05-format-des-fichiers.md)) puis lancez `publier`. Aucune nouvelle version du launcher n'est nécessaire : il relit `launcher.json` à chaque ouverture.

Exemple de news :

```json
{ "date": "2026-02-01", "titre": "Nouvelle carte", "texte": "Le serveur passe sur Tanoa ce soir.\n• Redémarrage à 21 h", "lien": { "nom": "Voir l'annonce", "url": "https://discord.gg/exemple" } }
```

Les news sont triées par date (la plus récente en haut) et affichées en texte brut : pas de HTML, mais `\n` passe à la ligne.

## Nouvelle version du launcher

1. Sur le PC Windows : montez `"version"` dans `launcher.config.json`, puis `npm run tauri build` (voir [03-signer-et-construire.md](03-signer-et-construire.md)).
2. Copiez **les deux fichiers** `…_x64-setup.exe` et `…_x64-setup.exe.sig` de `launcher/src-tauri/target/release/bundle/nsis/` dans `serveur/launcher-build/` sur le VPS :
   ```powershell
   scp "launcher\src-tauri\target\release\bundle\nsis\Arma Launcher_0.1.1_x64-setup.exe*" user@vps:arma-launcher/serveur/launcher-build/
   ```
3. Sur le VPS :
   ```sh
   docker compose run --rm publier --notes "Corrections diverses"
   ```

`publier` prend dans `launcher-build/` l'installeur signé de **plus haute version**, le copie dans `donnees/update/` sous deux noms (`<nom>_<version>_x64-setup.exe` pour la mise à jour automatique, `launcher-setup.exe` pour le bouton de la page d'accueil), supprime l'installeur de la version précédente et écrit `update/latest.json`. `URL_PUBLIQUE` (fichier `.env`) doit être renseignée : l'adresse de l'installeur y est absolue.

Au prochain démarrage, les launchers installés proposent « Mise à jour du launcher : la version X est disponible ».

Les anciens installeurs peuvent rester dans `launcher-build/` : seul le plus récent est publié.

## Automatiser depuis votre PC (exemple)

Un script PowerShell de quelques lignes suffit pour tout enchaîner :

```powershell
$vps = "user@play.example.com"
scp -r .\mods\* "${vps}:arma-launcher/serveur/mods/"
ssh $vps "cd arma-launcher/serveur && docker compose run --rm publier"
```

Pour de gros mods, préférez `rsync` (via WSL) qui ne renvoie que les fichiers modifiés.
