# Prepare tout ce qu'il faut envoyer sur le VPS : construit le launcher, assemble a-envoyer\ et a-envoyer.zip.
# Lance par preparer.bat (double-clic). Fichier en ASCII pur : Windows PowerShell 5.1 lit les .ps1 sans BOM en ANSI.
#
# Cle de signature des mises a jour (facultative), dans cet ordre :
#   1. variable TAURI_SIGNING_PRIVATE_KEY deja definie (contenu de la cle) ;
#   2. variable TAURI_SIGNING_PRIVATE_KEY_PATH (chemin du fichier .key) ;
#   3. fichier cles\launcher.key a la racine du depot (ignore par git).
# Mot de passe : TAURI_SIGNING_PRIVATE_KEY_PASSWORD, vide si absent.

$ErrorActionPreference = 'Stop'
$Racine = Split-Path -Parent $PSScriptRoot
$Launcher = Join-Path $Racine 'launcher'
$Serveur = Join-Path $Racine 'serveur'
$Sortie = Join-Path $Racine 'a-envoyer'
$Zip = Join-Path $Racine 'a-envoyer.zip'

function Etape([string]$texte) { Write-Host ''; Write-Host "== $texte" -ForegroundColor Cyan }
function Avertir([string]$texte) { Write-Host "ATTENTION : $texte" -ForegroundColor Yellow }
function Echec([string]$texte) { throw $texte }

function Ecrire-Texte([string]$chemin, [string[]]$lignes) {
    $texte = ($lignes -join "`r`n") + "`r`n"
    [System.IO.File]::WriteAllText($chemin, $texte, (New-Object System.Text.UTF8Encoding($false)))
}

function Exiger([string]$commande, [string]$nom, [string]$lien) {
    if (-not (Get-Command $commande -ErrorAction SilentlyContinue)) {
        Echec "$nom est introuvable. Installez-le depuis $lien puis relancez preparer.bat (fermez et rouvrez la fenetre apres l'installation)."
    }
}

function Lancer([string]$ligne, [string]$dossier) {
    Push-Location $dossier
    try {
        & cmd.exe /d /c "$ligne < nul"
        if ($LASTEXITCODE -ne 0) { Echec "la commande a echoue (code $LASTEXITCODE) : $ligne" }
    } finally {
        Pop-Location
    }
}

try {
    Etape 'Verification des outils'
    Exiger 'node' 'Node.js' 'https://nodejs.org (version LTS)'
    Exiger 'npm' 'npm (livre avec Node.js)' 'https://nodejs.org'
    Exiger 'cargo' 'Rust (cargo)' 'https://rustup.rs (puis les prerequis Tauri : https://tauri.app/start/prerequisites/)'
    Write-Host ("node {0}, npm {1}, {2}" -f (& node -v), (& cmd.exe /d /c 'npm -v'), (& cargo -V))

    Etape 'Verification de launcher.config.json'
    $cheminConfig = Join-Path $Racine 'launcher.config.json'
    $config = [System.IO.File]::ReadAllText($cheminConfig, [System.Text.Encoding]::UTF8) | ConvertFrom-Json
    if ([string]::IsNullOrWhiteSpace($config.urlServeur) -or $config.urlServeur -match 'example\.(com|org|net)') {
        Echec "launcher.config.json contient encore l'adresse d'exemple (urlServeur = $($config.urlServeur)). Mettez l'adresse de VOTRE serveur (ex. https://play.mon-serveur.fr), voir docs\02-personnaliser-le-launcher.md."
    }
    if ($config.identifiant -match '^com\.example\.') {
        Avertir "identifiant vaut encore $($config.identifiant) : choisissez le votre (ex. fr.monserveur.launcher) AVANT de distribuer le launcher, il ne doit plus changer ensuite."
    }
    Write-Host "Launcher : $($config.nom) $($config.version), serveur $($config.urlServeur)"

    Etape 'Cle de signature'
    $clePublique = [string]$config.clePubliqueMaj
    $cle = $env:TAURI_SIGNING_PRIVATE_KEY
    if (-not [string]::IsNullOrWhiteSpace($cle)) {
        Write-Host 'Cle privee prise dans TAURI_SIGNING_PRIVATE_KEY.'
    } else {
        $fichierCle = $env:TAURI_SIGNING_PRIVATE_KEY_PATH
        if ([string]::IsNullOrWhiteSpace($fichierCle)) { $fichierCle = Join-Path $Racine 'cles\launcher.key' }
        if (Test-Path -LiteralPath $fichierCle -PathType Leaf) {
            $cle = [System.IO.File]::ReadAllText($fichierCle).Trim()
            Write-Host "Cle privee chargee depuis $fichierCle."
        }
    }
    $signer = -not [string]::IsNullOrWhiteSpace($cle)
    if ([string]::IsNullOrWhiteSpace($clePublique)) {
        Avertir 'clePubliqueMaj est vide dans launcher.config.json : la mise a jour automatique du launcher sera desactivee (docs\03-signer-et-construire.md).'
        if ($signer) { Avertir 'une cle privee est presente mais inutilisee tant que clePubliqueMaj est vide.' }
        $signer = $false
    } elseif (-not $signer) {
        Avertir 'aucune cle privee (cles\launcher.key ou TAURI_SIGNING_PRIVATE_KEY) : installeur NON signe, la mise a jour automatique ne lui sera pas proposee. Les joueurs devront le telecharger sur la page du serveur.'
    }

    Etape 'Dependances npm'
    $verrou = Join-Path $Launcher 'node_modules\.package-lock.json'
    if (-not (Test-Path $verrou) -or (Get-Item (Join-Path $Launcher 'package-lock.json')).LastWriteTime -gt (Get-Item $verrou).LastWriteTime) {
        Lancer 'npm install' $Launcher
    } else {
        Write-Host 'Deja installees.'
    }

    Etape 'Construction du launcher (quelques minutes la premiere fois)'
    # Construit sans la signature integree de Tauri : sans mot de passe, elle attend une saisie au clavier
    # qui n'arrive jamais (docs\03, piege). La signature est faite juste apres par "tauri signer sign".
    $surcharge = Join-Path ([System.IO.Path]::GetTempPath()) 'preparer-sans-signature.json'
    Ecrire-Texte $surcharge @('{ "bundle": { "createUpdaterArtifacts": false } }')
    Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY, Env:TAURI_SIGNING_PRIVATE_KEY_PATH -ErrorAction SilentlyContinue
    $nsis = Join-Path $Launcher 'src-tauri\target\release\bundle\nsis'
    $nomInstalleur = "$($config.nom)_$($config.version)_x64-setup.exe"
    $installeur = Join-Path $nsis $nomInstalleur
    $signature = "$installeur.sig"
    foreach ($f in @($installeur, $signature)) { if (Test-Path -LiteralPath $f) { Remove-Item -LiteralPath $f -Force } }
    Lancer "npm run tauri build -- --config `"$surcharge`"" $Launcher
    if (-not (Test-Path -LiteralPath $installeur)) { Echec "installeur introuvable apres la construction : $installeur" }

    $signe = $false
    if ($signer) {
        Etape 'Signature de l''installeur'
        $env:TAURI_SIGNING_PRIVATE_KEY = $cle
        $motDePasse = ''
        if ([string]::IsNullOrEmpty($env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD)) { $motDePasse = ' -p ""' }
        try {
            Lancer "npx tauri signer sign$motDePasse `"$installeur`"" $Launcher
        } finally {
            Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY -ErrorAction SilentlyContinue
        }
        if (-not (Test-Path -LiteralPath $signature)) { Echec "la signature $nomInstalleur.sig n'a pas ete produite (mot de passe de la cle ? variable TAURI_SIGNING_PRIVATE_KEY_PASSWORD)." }
        $signe = $true
    }

    Etape 'Assemblage de a-envoyer'
    if (Test-Path $Sortie) { Remove-Item -LiteralPath $Sortie -Recurse -Force }
    if (Test-Path $Zip) { Remove-Item -LiteralPath $Zip -Force }
    New-Item -ItemType Directory -Path $Sortie | Out-Null
    foreach ($f in @('docker-compose.yml', '.env.exemple')) { Copy-Item -LiteralPath (Join-Path $Serveur $f) -Destination $Sortie }
    foreach ($d in @('nginx', 'https')) { Copy-Item -LiteralPath (Join-Path $Serveur $d) -Destination $Sortie -Recurse }
    New-Item -ItemType Directory -Path (Join-Path $Sortie 'publier') | Out-Null
    foreach ($f in @('Dockerfile', 'publier.py')) { Copy-Item -LiteralPath (Join-Path $Serveur "publier\$f") -Destination (Join-Path $Sortie 'publier') }

    $env_local = Join-Path $Serveur '.env'
    if (Test-Path -LiteralPath $env_local) {
        Copy-Item -LiteralPath $env_local -Destination (Join-Path $Sortie '.env')
        Write-Host '.env : copie de serveur\.env.'
    } else {
        $url = $config.urlServeur.TrimEnd('/')
        $domaine = ([System.Uri]$url).Host
        $lignes = [System.IO.File]::ReadAllLines((Join-Path $Serveur '.env.exemple')) | ForEach-Object {
            if ($_ -like 'URL_PUBLIQUE=*') { "URL_PUBLIQUE=$url" }
            elseif ($_ -like 'DOMAINE=*') { "DOMAINE=$domaine" }
            else { $_ }
        }
        [System.IO.File]::WriteAllText((Join-Path $Sortie '.env'), (($lignes -join "`n") + "`n"), (New-Object System.Text.UTF8Encoding($false)))
        Write-Host ".env : cree depuis .env.exemple (URL_PUBLIQUE=$url, DOMAINE=$domaine ; EMAIL_ACME a remplir)."
    }

    $configSortie = Join-Path $Sortie 'config'
    New-Item -ItemType Directory -Path $configSortie | Out-Null
    Copy-Item -LiteralPath (Join-Path $Serveur 'config\launcher.exemple.json') -Destination $configSortie
    $launcherLocal = Join-Path $Serveur 'config\launcher.json'
    if (Test-Path -LiteralPath $launcherLocal) {
        Copy-Item -LiteralPath $launcherLocal -Destination $configSortie
        Write-Host 'config\launcher.json : copie de serveur\config\launcher.json.'
    } else {
        Copy-Item -LiteralPath (Join-Path $Serveur 'config\launcher.exemple.json') -Destination (Join-Path $configSortie 'launcher.json')
        Write-Host 'config\launcher.json : cree depuis l''exemple (a editer : nom, adresse, port, liens, news, tfar).'
    }

    New-Item -ItemType Directory -Path (Join-Path $Sortie 'donnees') | Out-Null
    $mods = Join-Path $Sortie 'mods'
    New-Item -ItemType Directory -Path $mods | Out-Null
    Ecrire-Texte (Join-Path $mods 'LISEZMOI.txt') @(
        'Deposez ici les dossiers @Mod du serveur (ex. @CBA_A3, @ace, @task_force_radio),',
        'tels qu''ils sont dans le dossier d''Arma 3 du serveur de jeu.',
        'Seuls les dossiers dont le nom commence par @ sont publies. Ce fichier est ignore.',
        'Puis : docker compose run --rm publier'
    )

    $build = Join-Path $Sortie 'launcher-build'
    New-Item -ItemType Directory -Path $build | Out-Null
    Copy-Item -LiteralPath $installeur -Destination $build
    if ($signe) { Copy-Item -LiteralPath $signature -Destination $build }
    Ecrire-Texte (Join-Path $build 'VERSION.txt') @("$($config.nom) $($config.version)", $(if ($signe) { 'signe : mise a jour automatique proposee aux launchers installes' } else { 'non signe : page de telechargement seulement, pas de mise a jour automatique' }))

    Ecrire-Texte (Join-Path $Sortie 'LISEZMOI.txt') @(
        "$($config.nom) $($config.version) - fichiers a installer sur le VPS",
        '==========================================================================',
        '',
        '1. Envoyer ce dossier sur le VPS (WinSCP en glisser-deposer, ou depuis le PC :',
        '      scp -r a-envoyer utilisateur@vps:arma-launcher',
        '   ou envoyer a-envoyer.zip puis, sur le VPS : unzip a-envoyer.zip).',
        '   Le VPS doit avoir Docker et le plugin Compose : docs/01-installer-le-serveur.md.',
        '',
        '2. Sur le VPS, dans le dossier envoye, verifier deux fichiers :',
        "      .env                   URL_PUBLIQUE ($($config.urlServeur)), DOMAINE, EMAIL_ACME",
        '      config/launcher.json   nom du serveur, adresse et port de jeu, liens, news, tfar',
        '',
        '3. Deposer les dossiers @Mod dans mods/ (WinSCP, scp -r, rsync).',
        '',
        '4. Demarrer le serveur web, puis publier :',
        '      docker compose --profile https up -d     (HTTPS automatique avec Caddy)',
        '   ou docker compose up -d                     (derriere votre propre reverse proxy)',
        '      docker compose run --rm publier --a-sec  (simulation)',
        '      docker compose run --rm publier          (publication)',
        '',
        "5. Ouvrir $($config.urlServeur) : la page propose le launcher en telechargement.",
        '',
        'Mises a jour suivantes :',
        '- nouveaux mods, news, liens : modifier mods/ ou config/launcher.json sur le VPS,',
        '  puis docker compose run --rm publier. Rien a renvoyer depuis le PC.',
        '- nouvelle version du launcher : monter "version" dans launcher.config.json,',
        '  relancer preparer.bat, renvoyer seulement le dossier launcher-build/ (les deux',
        '  fichiers .exe et .exe.sig) dans launcher-build/ du VPS, puis publier.',
        '',
        $(if ($signe) { 'Ce launcher est signe : les launchers deja installes se mettront a jour tout seuls.' } else { 'ATTENTION : ce launcher n''est pas signe, il ne se mettra pas a jour tout seul (docs/03-signer-et-construire.md).' })
    )

    Etape 'Compression'
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    [System.IO.Compression.ZipFile]::CreateFromDirectory($Sortie, $Zip, [System.IO.Compression.CompressionLevel]::Optimal, $true)

    Write-Host ''
    Write-Host 'Termine.' -ForegroundColor Green
    Write-Host "  Dossier : $Sortie"
    Write-Host "  Archive : $Zip"
    Write-Host '  Suite   : lire a-envoyer\LISEZMOI.txt'
    exit 0
} catch {
    Write-Host ''
    Write-Host "ECHEC : $($_.Exception.Message)" -ForegroundColor Red
    exit 1
}
