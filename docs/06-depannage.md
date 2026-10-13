# 6. Dépannage

## Côté serveur

**`docker compose run --rm publier` : « launcher.json introuvable »**
Créez `serveur/config/launcher.json` depuis `config/launcher.exemple.json`.

**« aucun dossier @Mod »**
Les mods doivent être des dossiers dont le nom commence par `@`, directement dans `serveur/mods/` (pas dans un sous-dossier, pas en archive `.zip`).

**« URL_PUBLIQUE manquante »**
Un installeur signé est présent dans `launcher-build/` : renseignez `URL_PUBLIQUE` dans `serveur/.env`.

**La page d'accueil répond 403 ou 404**
Rien n'a encore été publié : lancez `publier`.

**Le certificat HTTPS n'est pas délivré (profil `https`)**
`docker compose logs caddy`. Causes habituelles : le DNS ne pointe pas encore vers le VPS, ports 80/443 fermés (pare-feu de l'hébergeur), `DOMAINE` vide dans `.env`, ou un autre programme occupe déjà le port 80/443 (`sudo ss -ltnp | grep -E ':80|:443'`).

**Les fichiers publiés appartiennent à root**
`publier` tourne en root dans son conteneur. C'est sans conséquence pour nginx (les fichiers sont lisibles par tous) ; pour les supprimer à la main, utilisez `sudo` ou `docker compose run --rm --entrypoint sh publier`.

**La publication est lente**
Au premier passage, `publier` calcule le SHA-256 de chaque fichier. Les passages suivants réutilisent un cache (taille + date de modification, dans `donnees/.publier/`, non servi par nginx) et ne recalculent que les fichiers modifiés.

## Côté launcher

**« Serveur de mise a jour injoignable et aucune configuration en cache »**
Le launcher n'a jamais pu lire `urlServeur/launcher.json`. Vérifiez dans un navigateur `https://play.example.com/launcher.json`, puis que `urlServeur` de `launcher.config.json` est la bonne adresse (elle est compilée dans le launcher : un changement demande une nouvelle construction).

**« Format launcher.json 2 inconnu »** ou **« liste de mods invalide »**
Voir les règles de validation dans [05-format-des-fichiers.md](05-format-des-fichiers.md). En général : `version` différent de `1`, ou aucun mod publié.

**« Manifeste refusé, chemin invalide : … »**
Le manifeste cite un fichier d'un mod absent de `launcher.json > mods`. Cela arrive si l'un des deux fichiers a été modifié à la main ou copié depuis une autre publication. Relancez `publier`, qui les réécrit ensemble.

**« … : contenu différent du manifeste »**
Le fichier téléchargé n'a pas la taille ou l'empreinte annoncée : publication en cours au même moment, ou cache intermédiaire (CDN, proxy) qui sert une ancienne version. Réessayer après la publication ; avec un CDN, voir la section CDN de [01-installer-le-serveur.md](01-installer-le-serveur.md).

**« Arma3_x64.exe et Arma3BattlEye.exe introuvables dans ce dossier »**
Le joueur doit choisir le dossier d'installation d'Arma 3 (celui qui contient `Arma3_x64.exe`), en général `C:\Program Files (x86)\Steam\steamapps\common\Arma 3`. Le launcher le trouve tout seul via le registre si Arma 3 a été installé par Steam.

**Le jeu démarre mais ne rejoint pas le serveur**
Vérifiez `serveur.hote` et `serveur.port` dans `launcher.json`, et le mot de passe. Les signatures : si le serveur exige `verifySignatures = 2`, les mods publiés doivent être exactement ceux du serveur, avec leurs `.bisign`, et les `.bikey` correspondantes dans `keys/` côté serveur.

**Le launcher affiche « Serveur hors ligne » alors qu'il tourne**
L'état vient d'une requête A2S (Steam) sur **port de jeu + 1** en UDP. Si ce port est filtré, le launcher n'a pas de réponse ; il laisse quand même jouer (seul `etatServeur` peut retenir le bouton JOUER).

**Aucune proposition de mise à jour du launcher**
- `clePubliqueMaj` était vide lors de la construction du launcher installé : il ne cherche jamais de mise à jour. Les joueurs doivent installer une fois une version construite avec la clé.
- La version de `update/latest.json` n'est pas supérieure à la version installée.
- La signature ne correspond pas : l'installeur n'a pas été signé avec la clé dont la partie publique est dans le launcher installé. La mise à jour échoue alors avec une erreur de signature dans la fenêtre « Mise à jour du launcher ».

**La construction reste bloquée sur « Decrypting updater signing key »**
Voir [03-signer-et-construire.md](03-signer-et-construire.md#piège--construction-bloquée-sur-le-mot-de-passe).

**« A public key has been found, but no private key »**
`clePubliqueMaj` est renseignée mais `TAURI_SIGNING_PRIVATE_KEY` n'est pas définie dans le terminal qui construit.

**Windows SmartScreen bloque l'installeur**
Normal sans certificat de signature de code : *Informations complémentaires* > *Exécuter quand même*. Voir la fin de [03-signer-et-construire.md](03-signer-et-construire.md).

**L'antivirus met un fichier de mod en quarantaine**
Le launcher retéléchargera le fichier manquant à la vérification suivante. Ajoutez le dossier des mods aux exclusions de l'antivirus si cela se répète.

## Journaux

- Serveur : `docker compose logs nginx` (accès, avec le `User-Agent` défini dans `launcher.config.json`), `docker compose logs caddy`.
- Launcher : les erreurs s'affichent dans la fenêtre. En développement (`npm run tauri dev`), la console du terminal et les outils de développement de la WebView (clic droit > Inspecter) donnent le détail.
