# Contribuer

Merci de votre intérêt. Quelques règles simples :

- **Une modification = une demande de fusion** (pull request) courte, avec une description de ce qu'elle change et pourquoi.
- **Les tests doivent passer** avant d'ouvrir la demande :
  ```sh
  cd launcher
  npm ci
  npm run configurer -- --verifier
  npm run build
  cd src-tauri && cargo test --lib
  ```
  Pour le serveur : `python -m unittest discover -s serveur/publier`, `cd serveur && docker compose config --quiet`, puis une publication d'essai sur un faux dossier de mods (`docker compose run --rm publier --a-sec`).
- **Essai de bout en bout** (synchronisation réelle contre le serveur Docker) : démarrez `serveur/` en local (`docker compose up -d`, port `127.0.0.1:8080`), déposez un ou deux faux `@Mod`, publiez, puis :
  ```powershell
  $env:ESSAI_SERVEUR = "http://127.0.0.1:8080"; $env:ESSAI_MODS = "$env:TEMP\mods-essai"
  cd launcher/src-tauri; cargo test --lib essai -- --ignored --nocapture
  ```
  Le test (`src/essai.rs`) enchaîne les fonctions des commandes `verifier` et `mettre_a_jour` et affiche ce qu'il a téléchargé ou supprimé. Modifiez un fichier côté serveur, republiez, relancez : un seul téléchargement, et `docker compose logs nginx` ne montre qu'un `GET /mods/…`.
- **Rien de propre à un serveur** dans le code : nom, adresse et couleurs passent par `launcher.config.json` ; news, liens et serveur de jeu par `serveur/config/launcher.json`.
- **Jamais de secret** dans un commit : clé de signature (`*.key`), `.env`, mot de passe de serveur.
- **Formats** : toute évolution de `launcher.json` ou `manifest.json` doit rester lisible par les launchers déjà installés (champs nouveaux facultatifs) et être documentée dans `docs/05-format-des-fichiers.md`.
- Le code et la documentation sont en français ; gardez ce choix pour la cohérence.

Signaler un bug : décrivez la version du launcher, ce que vous attendiez, ce qui s'est passé, et joignez si possible le `launcher.json` publié (sans mot de passe).
