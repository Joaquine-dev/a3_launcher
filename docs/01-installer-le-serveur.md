# 1. Installer le serveur de mises à jour

Le serveur de mises à jour ne fait que servir des fichiers : le launcher y lit `launcher.json`, `manifest.json`, les fichiers de mods et `update/latest.json`. Il peut tourner sur la même machine que le serveur Arma 3 ou sur une autre.

## Ce qu'il faut

- Un VPS Linux (Debian, Ubuntu…). 1 vCPU et 1 Go de RAM suffisent : nginx sert des fichiers statiques.
- De l'espace disque : environ **deux fois** la taille de vos mods (le dossier `mods/` que vous déposez, et sa copie publiée dans `donnees/`).
- De la bande passante : chaque nouveau joueur télécharge la totalité des mods, puis seulement ce qui change.
- Un nom de domaine (ou un sous-domaine), par exemple `play.example.com`.

## 1. Installer Docker

Sur Debian ou Ubuntu, le script officiel installe Docker Engine et le plugin Compose :

```sh
curl -fsSL https://get.docker.com | sh
sudo usermod -aG docker $USER   # puis se déconnecter et se reconnecter
docker compose version          # doit afficher une version 2.x
```

## 2. Le DNS

Créez chez votre registraire un enregistrement **A** (et **AAAA** si le VPS a une IPv6) :

```
play.example.com.   A   203.0.113.10
```

Vérifiez avant d'aller plus loin : `ping play.example.com` doit répondre depuis l'adresse du VPS. Sans cela, Let's Encrypt ne pourra pas délivrer de certificat.

Ouvrez les ports **80** et **443** (TCP, et UDP 443 pour HTTP/3) dans le pare-feu du VPS et celui de l'hébergeur.

## 3. Récupérer le dépôt et le configurer

```sh
git clone <url-du-dépôt> arma-launcher
cd arma-launcher/serveur
cp .env.exemple .env
cp config/launcher.exemple.json config/launcher.json
```

Éditez `.env` :

| Variable | Rôle |
|---|---|
| `URL_PUBLIQUE` | Adresse publique, sans `/` final : `https://play.example.com`. Doit être identique à `urlServeur` de `launcher.config.json`. Sert à écrire l'URL de l'installeur dans `update/latest.json`. |
| `HTTP_ECOUTE` | Où nginx écoute sur l'hôte : `80` (nginx exposé directement) ou `127.0.0.1:8080` (derrière un reverse proxy ou avec le profil `https`). |
| `DOMAINE`, `EMAIL_ACME` | Utilisés seulement par le profil `https` (Caddy). |

Éditez `config/launcher.json` (nom du serveur, adresse et port de jeu, liens, news) : voir [05-format-des-fichiers.md](05-format-des-fichiers.md). Ce fichier n'est pas suivi par git.

## 4. HTTPS

Le launcher n'accepte que des liens `https://` pour l'affichage, et la mise à jour automatique doit passer par HTTPS. Trois voies, choisissez-en **une**.

### Voie A : le profil `https` fourni (le plus simple)

Caddy se place devant nginx, obtient et renouvelle le certificat Let's Encrypt tout seul.

Dans `.env` :

```
HTTP_ECOUTE=127.0.0.1:8080
DOMAINE=play.example.com
EMAIL_ACME=vous@example.com
```

Puis :

```sh
docker compose --profile https up -d
docker compose logs -f caddy      # attendre « certificate obtained successfully »
```

À chaque commande `up`, pensez à `--profile https` (ou mettez `COMPOSE_PROFILES=https` dans `.env`).

### Voie B : derrière un reverse proxy existant

Si le VPS a déjà un Caddy, un Traefik ou un nginx sur les ports 80/443, laissez-le gérer le certificat et relayez vers nginx :

`.env` : `HTTP_ECOUTE=127.0.0.1:8080`, puis `docker compose up -d`.

**Caddy** (Caddyfile de l'hôte) :

```
play.example.com {
	reverse_proxy 127.0.0.1:8080
}
```

**nginx de l'hôte** :

```nginx
server {
    listen 443 ssl http2;
    server_name play.example.com;
    ssl_certificate     /etc/letsencrypt/live/play.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/play.example.com/privkey.pem;
    client_max_body_size 1m;
    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header Host $host;
        proxy_buffering off;          # gros fichiers : pas de tampon disque côté proxy
    }
}
```

**Traefik** (conteneur Traefik sur le même réseau Docker) : ne publiez aucun port pour nginx et ajoutez des labels via un fichier `docker-compose.override.yml` :

```yaml
services:
  nginx:
    ports: !reset []
    networks: [default, traefik]
    labels:
      - traefik.enable=true
      - traefik.http.routers.launcher.rule=Host(`play.example.com`)
      - traefik.http.routers.launcher.entrypoints=websecure
      - traefik.http.routers.launcher.tls.certresolver=letsencrypt
      - traefik.http.services.launcher.loadbalancer.server.port=80
networks:
  traefik:
    external: true
```

Le reverse proxy ne doit ni réécrire `Cache-Control` ni mettre en cache `launcher.json`, `manifest.json` et `update/latest.json`.

### Voie C : certbot et nginx seul

Pour ne rien ajouter d'autre : nginx sert déjà `/.well-known/acme-challenge/` depuis `donnees/`.

1. `.env` : `HTTP_ECOUTE=80`, `docker compose up -d`.
2. Sur l'hôte : `sudo apt install certbot` puis
   ```sh
   sudo certbot certonly --webroot -w "$PWD/donnees" -d play.example.com
   ```
3. Ajoutez un bloc 443 à `nginx/default.conf` (copie du bloc existant avec `listen 443 ssl;`, `ssl_certificate /etc/letsencrypt/live/play.example.com/fullchain.pem;` et `ssl_certificate_key …/privkey.pem;`), montez `/etc/letsencrypt` en lecture seule dans le service `nginx` et publiez le port `443:443`.
4. Renouvellement : `certbot renew --deploy-hook "docker compose -f $PWD/docker-compose.yml exec nginx nginx -s reload"`, déjà programmé par le paquet certbot (minuteur systemd).

La voie A fait la même chose sans ces étapes : préférez-la si vous n'avez pas de raison particulière.

## 5. Vérifier

```sh
docker compose ps                       # nginx « healthy »
curl -I https://play.example.com/sante  # 200
```

La première publication (voir [04-publier-une-mise-a-jour.md](04-publier-une-mise-a-jour.md)) crée `launcher.json`, `manifest.json` et la page d'accueil. Avant, `https://play.example.com/` répond 403 ou 404 : c'est normal.

## CDN (Cloudflare…)

Possible, avec deux précautions :

- les fichiers de mods sont envoyés avec `Cache-Control: public, max-age=86400, s-maxage=300` : un CDN les garde au plus 5 minutes. Ne forcez pas un cache plus long par une règle du CDN, sinon un joueur pourrait recevoir un ancien fichier, refusé par le launcher (empreinte différente) ;
- `launcher.json`, `manifest.json`, `update/latest.json` et `index.html` sont en `no-cache` : ne les mettez jamais en cache côté CDN.

## Ce que fait la configuration nginx

`serveur/nginx/default.conf` :

- pas de listing de répertoires (`autoindex off`), fichiers cachés et fichiers temporaires de publication en 404 ;
- `sendfile`, requêtes `Range` (reprise, téléchargement partiel) ;
- types MIME : tout ce qui est sous `/mods/`, `/tfar/` et `/update/` (hors `latest.json`) part en `application/octet-stream` (`.pbo`, `.bisign`, `.bikey`, `.ts3_plugin`, `.exe`…) ;
- en-têtes de sécurité (`nosniff`, `X-Frame-Options`, `Referrer-Policy`, CSP stricte sur la page d'accueil) ;
- `/sante` pour le healthcheck Docker.
