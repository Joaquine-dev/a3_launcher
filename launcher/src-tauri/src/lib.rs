mod a2s;
mod arma;
mod config;
mod controles;
mod deplacer;
mod distant;
mod garde;
mod hash;
mod manifest;
mod sync;
mod tfar;

use serde::Serialize;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Mutex;

const LARGEUR: f64 = 1100.0;
const HAUTEUR: f64 = 680.0;

struct Etat {
    reglages: config::Reglages,
    launcher: Option<distant::LauncherJson>,
    delta: Option<manifest::Delta>,
    hors_ligne: bool,
    client: reqwest::Client,
}

struct Partage(Mutex<Etat>);

#[derive(Serialize)]
struct Accueil {
    reglages: config::Reglages,
    launcher: Option<distant::LauncherJson>,
    hors_ligne: bool,
    erreur: Option<String>,
    arma_detecte: bool,
}

#[derive(Serialize)]
struct Resume {
    fichiers: usize,
    octets: u64,
    a_supprimer: usize,
    pret: bool,
    hors_ligne: bool,
    espace_manquant: u64,
}

#[derive(Serialize)]
struct Veille {
    info: Option<a2s::InfoServeur>,
    garde: garde::Garde,
    reboot_dans: Option<i64>,
}

#[derive(Serialize)]
struct Controles {
    teamspeak: bool,
    tfar: tfar::Etat,
}

fn chemin_config() -> PathBuf {
    config::dossier_appli().join("config.json")
}

fn chemin_cache() -> PathBuf {
    config::dossier_appli().join("hash-cache.json")
}

fn accueil(e: &Etat, erreur: Option<String>) -> Accueil {
    Accueil {
        reglages: e.reglages.clone(),
        launcher: e.launcher.clone(),
        hors_ligne: e.hors_ligne,
        erreur,
        arma_detecte: e.reglages.dossier_arma.as_deref().is_some_and(arma::dossier_valide),
    }
}

fn ajuster_fenetre(fenetre: &tauri::WebviewWindow) {
    let Ok(Some(ecran)) = fenetre.current_monitor() else { return };
    let zone = ecran.work_area().size.to_logical::<f64>(ecran.scale_factor());
    let mut large = (zone.width * 0.94).min(LARGEUR);
    let mut haut = large * HAUTEUR / LARGEUR;
    let haut_max = zone.height * 0.94;
    if haut > haut_max {
        haut = haut_max;
        large = haut * LARGEUR / HAUTEUR;
    }
    let _ = fenetre.set_size(tauri::LogicalSize::new(large, haut));
    let _ = fenetre.center();
}

fn afficher(app: &AppHandle) {
    if let Some(fenetre) = app.get_webview_window("main") {
        let _ = fenetre.show();
        let _ = fenetre.unminimize();
        let _ = fenetre.set_focus();
    }
}

#[tauri::command]
async fn charger(partage: State<'_, Partage>) -> Result<Accueil, String> {
    let mut e = partage.0.lock().await;
    if !e.reglages.dossier_arma.as_deref().is_some_and(arma::dossier_valide) {
        if let Some(dossier) = arma::dossier_registre() {
            e.reglages.dossier_arma = Some(dossier);
            let _ = config::sauver(&chemin_config(), &e.reglages);
        }
    }
    let copie = config::dossier_appli().join("launcher.json");
    let texte = match distant::telecharger_texte(&e.client, "launcher.json").await {
        Ok(t) => {
            e.hors_ligne = false;
            Some(t)
        }
        Err(_) => {
            e.hors_ligne = true;
            std::fs::read_to_string(&copie).ok()
        }
    };
    e.launcher = None;
    e.delta = None;
    let mut erreur = None;
    if let Some(t) = texte {
        match distant::analyser_launcher(&t) {
            Ok(l) => {
                if !e.hors_ligne {
                    let _ = std::fs::create_dir_all(config::dossier_appli());
                    let _ = std::fs::write(&copie, &t);
                }
                e.launcher = Some(l);
            }
            Err(m) => erreur = Some(m),
        }
    } else {
        erreur = Some("Serveur de mise a jour injoignable et aucune configuration en cache.".into());
    }
    Ok(accueil(&e, erreur))
}

async fn verifier_interne(partage: &Partage, complet: bool) -> Result<Resume, String> {
    let mut e = partage.0.lock().await;
    if !e.hors_ligne {
        if let Ok(t) = distant::telecharger_texte(&e.client, "launcher.json").await {
            if let Ok(l) = distant::analyser_launcher(&t) {
                let _ = std::fs::create_dir_all(config::dossier_appli());
                let _ = std::fs::write(config::dossier_appli().join("launcher.json"), &t);
                e.launcher = Some(l);
            }
        }
    }
    let launcher = e.launcher.clone().ok_or("launcher.json indisponible")?;
    let racine = e.reglages.dossier_mods.clone();
    sync::nettoyer_parts(&racine, &launcher.mods);
    if e.hors_ligne {
        e.delta = None;
        let pret = launcher.mods.iter().all(|m| racine.join(m).is_dir());
        return Ok(Resume { fichiers: 0, octets: 0, a_supprimer: 0, pret, hors_ligne: true, espace_manquant: 0 });
    }
    let texte = match distant::telecharger_texte(&e.client, "manifest.json").await {
        Ok(t) => t,
        Err(_) => {
            e.hors_ligne = true;
            e.delta = None;
            let pret = launcher.mods.iter().all(|m| racine.join(m).is_dir());
            return Ok(Resume { fichiers: 0, octets: 0, a_supprimer: 0, pret, hors_ligne: true, espace_manquant: 0 });
        }
    };
    let manifeste = distant::analyser_manifeste(&texte, &launcher.mods)?;
    let mods = launcher.mods.clone();
    let dossier_mods = racine.clone();
    let delta = tauri::async_runtime::spawn_blocking(move || {
        let mut cache = if complet { hash::CacheHash::default() } else { hash::CacheHash::charger(&chemin_cache()) };
        let locaux = sync::lister_locaux(&racine, &mods);
        let delta = manifest::calculer_delta(&manifeste, &locaux, &mods, |c| cache.hash(&racine, c));
        let _ = cache.sauver(&chemin_cache());
        delta
    })
    .await
    .map_err(|x| x.to_string())?;
    let resume = Resume {
        fichiers: delta.a_telecharger.len(),
        octets: delta.octets(),
        a_supprimer: delta.a_supprimer.len(),
        pret: delta.vide(),
        hors_ligne: false,
        espace_manquant: controles::espace_manquant(controles::espace_libre(&dossier_mods), delta.octets()),
    };
    e.delta = Some(delta);
    Ok(resume)
}

#[tauri::command]
async fn verifier(complet: bool, partage: State<'_, Partage>) -> Result<Resume, String> {
    verifier_interne(&partage, complet).await
}

#[tauri::command]
async fn mettre_a_jour(app: AppHandle, partage: State<'_, Partage>) -> Result<Resume, String> {
    let (delta, racine, client) = {
        let e = partage.0.lock().await;
        (e.delta.clone().ok_or("Verification requise avant la mise a jour")?, e.reglages.dossier_mods.clone(), e.client.clone())
    };
    let manque = controles::espace_manquant(controles::espace_libre(&racine), delta.octets());
    if manque > 0 {
        return Err(format!("Espace disque insuffisant : il manque {} Mo sur le disque des mods", manque / 1024 / 1024 + 1));
    }
    let emetteur = app.clone();
    let resultat = sync::synchroniser(&client, distant::BASE, &racine, &delta, move |p| {
        let _ = emetteur.emit("progression", p);
    })
    .await;
    let resume = verifier_interne(&partage, false).await;
    resultat?;
    resume
}

#[tauri::command]
async fn jouer(app: AppHandle, partage: State<'_, Partage>) -> Result<(), String> {
    let e = partage.0.lock().await;
    let l = e.launcher.as_ref().ok_or("launcher.json indisponible")?;
    let pret = if e.hors_ligne {
        l.mods.iter().all(|m| e.reglages.dossier_mods.join(m).is_dir())
    } else {
        e.delta.as_ref().is_some_and(|d| d.vide())
    };
    if !pret {
        return Err("Les mods ne sont pas a jour".into());
    }
    let dossier = e.reglages.dossier_arma.clone().filter(|d| arma::dossier_valide(d)).ok_or("Dossier Arma 3 introuvable")?;
    let args = arma::arguments(&arma::Lancement {
        dossier_mods: &e.reglages.dossier_mods,
        mods: &l.mods,
        parametres_serveur: &l.parametres,
        parametres_joueur: &e.reglages.parametres,
        hote: &l.serveur.hote,
        port: l.serveur.port,
        mot_de_passe: &l.serveur.mot_de_passe,
    });
    arma::lancer(&dossier, &args).map_err(|x| format!("Lancement impossible : {x}"))?;
    if e.reglages.rejoindre_teamspeak && controles::teamspeak_installe() {
        if let Some(url) = controles::lien_teamspeak(&l.liens).map(str::to_owned) {
            tauri::async_runtime::spawn_blocking(move || {
                if !arma::processus_en_cours("ts3client_win64.exe") {
                    let _ = ouvrir_url(url);
                }
            });
        }
    }
    let apres = e.reglages.apres_lancement.clone();
    drop(e);
    if apres == config::ApresLancement::Fermer {
        app.exit(0);
        return Ok(());
    }
    if let Some(fenetre) = app.get_webview_window("main") {
        let _ = fenetre.hide();
    }
    tauri::async_runtime::spawn(async move {
        let debut = Instant::now();
        let mut vu = false;
        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;
            let actif = tauri::async_runtime::spawn_blocking(arma::arma_en_cours).await.unwrap_or(false);
            vu |= actif;
            if (vu && !actif) || (!vu && debut.elapsed() > Duration::from_secs(180)) {
                break;
            }
        }
        afficher(&app);
    });
    Ok(())
}

#[tauri::command]
async fn veille_serveur(hote: String, port: u16, partage: State<'_, Partage>) -> Result<Veille, String> {
    let (client, etat) = {
        let e = partage.0.lock().await;
        (e.client.clone(), e.launcher.as_ref().map(|l| l.etat_serveur.clone()).unwrap_or_default())
    };
    let site = async {
        if etat.is_empty() {
            return None;
        }
        tokio::time::timeout(Duration::from_secs(5), distant::telecharger_texte(&client, &etat)).await.ok().and_then(Result::ok)
    };
    let (info, site) = tokio::join!(a2s::interroger(&hote, port.saturating_add(1), Duration::from_secs(3)), site);
    let site = site.and_then(|t| garde::analyser(&t));
    let garde = garde::garde(info.is_some(), site.as_ref());
    let reboot_dans = site.and_then(|s| s.reboot_dans).filter(|s| *s > 0);
    Ok(Veille { info, garde, reboot_dans })
}

#[tauri::command]
async fn controles(partage: State<'_, Partage>) -> Result<Controles, ()> {
    let (version, attendu) = partage.0.lock().await.launcher.as_ref().map(|l| (l.tfar_version.clone(), l.tfar_dll.clone())).unwrap_or_default();
    let tfar = tauri::async_runtime::spawn_blocking(move || tfar::etat(&version, &attendu)).await.unwrap_or(tfar::Etat::Absent);
    Ok(Controles { teamspeak: controles::teamspeak_installe(), tfar })
}

#[tauri::command]
async fn sauver_reglages(app: AppHandle, reglages: config::Reglages, partage: State<'_, Partage>) -> Result<Accueil, String> {
    if !reglages.dossier_mods.is_absolute() {
        return Err("Le dossier des mods doit etre un chemin complet".into());
    }
    if let Some(dossier) = &reglages.dossier_arma {
        if !arma::dossier_valide(dossier) {
            return Err("Arma3_x64.exe et Arma3BattlEye.exe introuvables dans ce dossier".into());
        }
    }
    let (ancien, mods) = {
        let e = partage.0.lock().await;
        (e.reglages.dossier_mods.clone(), e.launcher.as_ref().map(|l| l.mods.clone()).unwrap_or_default())
    };
    let nouveau = reglages.dossier_mods.clone();
    if !deplacer::a_deplacer(&ancien, &nouveau, &mods).is_empty() && !deplacer::meme_dossier(&ancien, &nouveau) {
        if arma::arma_en_cours() {
            return Err("Fermez Arma 3 avant de deplacer les mods".into());
        }
        let libre = controles::espace_libre(&nouveau);
        tauri::async_runtime::spawn_blocking(move || {
            let mut dernier = 0u64;
            deplacer::deplacer(&ancien, &nouveau, &mods, libre, &mut |fait, total| {
                if fait - dernier >= 8 * 1024 * 1024 || fait == total {
                    dernier = fait;
                    let _ = app.emit("progression", sync::Progression { fait, total });
                }
            })
        })
        .await
        .map_err(|x| x.to_string())??;
    }
    let mut e = partage.0.lock().await;
    config::sauver(&chemin_config(), &reglages).map_err(|x| x.to_string())?;
    e.reglages = reglages;
    e.delta = None;
    Ok(accueil(&e, None))
}

#[tauri::command]
async fn tfar_installer(partage: State<'_, Partage>) -> Result<(), String> {
    let (client, plugin) = {
        let e = partage.0.lock().await;
        let plugin = e.launcher.as_ref().map(|l| l.tfar_plugin.clone()).filter(|p| !p.is_empty()).ok_or("Plugin TFAR non publie")?;
        (e.client.clone(), plugin)
    };
    let octets = distant::telecharger_octets(&client, &plugin).await?;
    let destination = std::env::temp_dir().join("task_force_radio.ts3_plugin");
    std::fs::write(&destination, octets).map_err(|x| x.to_string())?;
    tauri_plugin_opener::open_path(destination.to_string_lossy().into_owned(), None::<&str>).map_err(|x| x.to_string())
}

#[tauri::command]
fn ouvrir_lien(url: String) -> Result<(), String> {
    if !distant::lien_autorise(&url) {
        return Err("Lien refuse".into());
    }
    ouvrir_url(url)
}

const TEAMSPEAK_ABSENT: &str = "teamspeak_absent";

fn ouvrir_url(url: String) -> Result<(), String> {
    if url.to_ascii_lowercase().starts_with("ts3server://") {
        if let Some(client) = controles::client_teamspeak() {
            return controles::lancer_par_le_shell(&client, &url).map_err(|x| format!("TeamSpeak ne demarre pas : {x}"));
        }
        if !controles::association_ts3server_valide() {
            return Err(TEAMSPEAK_ABSENT.into());
        }
    }
    tauri_plugin_opener::open_url(url, None::<&str>).map_err(|x| x.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let client = reqwest::Client::builder()
        .user_agent(concat!(env!("LAUNCHER_USER_AGENT"), "/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(10))
        .read_timeout(Duration::from_secs(30))
        .build()
        .expect("client HTTP");
    let etat = Etat { reglages: config::charger(&chemin_config()), launcher: None, delta: None, hors_ligne: false, client };

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| afficher(app)))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(Partage(Mutex::new(etat)))
        .setup(|app| {
            if let Some(fenetre) = app.get_webview_window("main") {
                ajuster_fenetre(&fenetre);
                let _ = fenetre.show();
            }
            let ouvrir = MenuItem::with_id(app, "afficher", "Afficher", true, None::<&str>)?;
            let quitter = MenuItem::with_id(app, "quitter", "Quitter", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&ouvrir, &quitter])?;
            let mut icone = TrayIconBuilder::new().tooltip(env!("LAUNCHER_NOM")).menu(&menu).on_menu_event(|app, evenement| match evenement.id.as_ref() {
                "afficher" => afficher(app),
                "quitter" => app.exit(0),
                _ => {}
            });
            if let Some(image) = app.default_window_icon() {
                icone = icone.icon(image.clone());
            }
            icone.build(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            charger,
            verifier,
            mettre_a_jour,
            jouer,
            veille_serveur,
            controles,
            sauver_reglages,
            tfar_installer,
            ouvrir_lien
        ])
        .run(tauri::generate_context!())
        .expect("erreur au lancement du launcher");
}
