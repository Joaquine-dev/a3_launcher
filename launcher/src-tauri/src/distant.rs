use crate::manifest::{self, Manifeste};
use serde::{Deserialize, Serialize};

/// URL du serveur de mises a jour, lue dans launcher.config.json par build.rs.
pub const BASE: &str = env!("LAUNCHER_URL_SERVEUR");
const VERSION_FORMAT: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Serveur {
    pub nom: String,
    pub hote: String,
    pub port: u16,
    #[serde(rename = "motDePasse", default)]
    pub mot_de_passe: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lien {
    pub nom: String,
    pub url: String,
    #[serde(default)]
    pub icone: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct News {
    pub date: String,
    pub titre: String,
    pub texte: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lien: Option<Lien>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LauncherJson {
    pub version: u32,
    pub serveur: Serveur,
    pub mods: Vec<String>,
    #[serde(default)]
    pub parametres: String,
    #[serde(default)]
    pub liens: Vec<Lien>,
    #[serde(default)]
    pub news: Vec<News>,
    #[serde(rename = "tfarPlugin", default)]
    pub tfar_plugin: String,
    #[serde(rename = "tfarDll", default)]
    pub tfar_dll: String,
    #[serde(rename = "etatServeur", default)]
    pub etat_serveur: String,
}

pub fn lien_autorise(url: &str) -> bool {
    let bas = url.to_ascii_lowercase();
    ["https://", "ts3server://", "discord://"].iter().any(|p| bas.starts_with(p))
}

fn relatif_sur(chemin: &str) -> bool {
    !chemin.is_empty()
        && !chemin.starts_with('/')
        && !chemin.contains('\\')
        && !chemin.contains(':')
        && chemin.split('/').all(|s| !s.is_empty() && s != "." && s != "..")
}

pub fn analyser_launcher(texte: &str) -> Result<LauncherJson, String> {
    let mut l: LauncherJson = serde_json::from_str(texte).map_err(|e| format!("launcher.json illisible : {e}"))?;
    if l.version != VERSION_FORMAT {
        return Err(format!("Format launcher.json {} inconnu : mettez a jour le launcher", l.version));
    }
    if l.mods.is_empty() || l.mods.iter().any(|m| !m.starts_with('@') || !relatif_sur(m) || m.contains('/')) {
        return Err("launcher.json : liste de mods invalide".into());
    }
    if !l.tfar_plugin.is_empty() && !relatif_sur(&l.tfar_plugin) {
        return Err("launcher.json : chemin du plugin TFAR invalide".into());
    }
    if !l.etat_serveur.is_empty() && !relatif_sur(l.etat_serveur.split('?').next().unwrap_or("")) {
        return Err("launcher.json : chemin etatServeur invalide".into());
    }
    if !l.tfar_dll.is_empty() && (l.tfar_dll.len() != 64 || !l.tfar_dll.bytes().all(|b| b.is_ascii_hexdigit())) {
        return Err("launcher.json : empreinte du plugin TFAR invalide".into());
    }
    l.liens.retain(|lien| lien_autorise(&lien.url));
    Ok(l)
}

pub fn analyser_manifeste(texte: &str, mods: &[String]) -> Result<Manifeste, String> {
    let m: Manifeste = serde_json::from_str(texte).map_err(|e| format!("manifest.json illisible : {e}"))?;
    manifest::valider(&m, mods)?;
    Ok(m)
}

pub async fn telecharger_octets(client: &reqwest::Client, chemin: &str) -> Result<Vec<u8>, String> {
    let url = format!("{BASE}/{chemin}");
    let reponse = client.get(&url).send().await.and_then(|r| r.error_for_status()).map_err(|e| format!("{url} : {e}"))?;
    reponse.bytes().await.map(|b| b.to_vec()).map_err(|e| format!("{url} : {e}"))
}

pub async fn telecharger_texte(client: &reqwest::Client, chemin: &str) -> Result<String, String> {
    let octets = telecharger_octets(client, chemin).await?;
    String::from_utf8(octets).map_err(|e| format!("{chemin} : {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXEMPLE: &str = r#"{
      "version": 1,
      "serveur": { "nom": "Mon Serveur", "hote": "play.example.com", "port": 2302, "motDePasse": "" },
      "mods": ["@CBA_A3", "@task_force_radio", "@mon_mod"],
      "parametres": "-skipIntro -noSplash",
      "liens": [
        { "nom": "TeamSpeak", "url": "ts3server://play.example.com?port=9987", "icone": "teamspeak" },
        { "nom": "Piege", "url": "file:///C:/Windows/system32/calc.exe", "icone": "site" }
      ],
      "news": [{ "date": "2026-09-17", "titre": "T", "texte": "<b>brut</b>" }],
      "tfarPlugin": "tfar/task_force_radio.ts3_plugin"
    }"#;

    #[test]
    fn lit_l_exemple_et_retire_les_liens_interdits() {
        let l = analyser_launcher(EXEMPLE).unwrap();
        assert_eq!(l.serveur.port, 2302);
        assert_eq!(l.mods.len(), 3);
        assert_eq!(l.liens.len(), 1);
        assert_eq!(l.liens[0].nom, "TeamSpeak");
        assert_eq!(l.news[0].texte, "<b>brut</b>");
    }

    #[test]
    fn le_mot_de_passe_publie_arrive_sur_la_ligne_de_lancement() {
        let l = analyser_launcher(&EXEMPLE.replace("\"motDePasse\": \"\"", "\"motDePasse\": \"secret\"")).unwrap();
        assert_eq!(l.serveur.mot_de_passe, "secret");
        let dossier = std::path::PathBuf::from(r"D:m");
        let args = crate::arma::arguments(&crate::arma::Lancement {
            dossier_mods: &dossier,
            mods: &l.mods,
            parametres_serveur: &l.parametres,
            parametres_joueur: "",
            hote: &l.serveur.hote,
            port: l.serveur.port,
            mot_de_passe: &l.serveur.mot_de_passe,
        });
        assert!(args.contains(&format!("-password={}", l.serveur.mot_de_passe)));
    }

    #[test]
    fn l_exemple_du_serveur_est_accepte() {
        let texte = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../serveur/config/launcher.exemple.json")).unwrap();
        // La liste des mods est ajoutee par serveur/publier, pas par le fichier edite a la main.
        let mut v: serde_json::Value = serde_json::from_str(&texte).unwrap();
        v["mods"] = serde_json::json!(["@CBA_A3", "@mon_mod"]);
        let l = analyser_launcher(&v.to_string()).unwrap();
        assert_eq!(l.liens.len(), 3);
        assert!(l.serveur.mot_de_passe.is_empty(), "l'exemple ne doit contenir aucun mot de passe");
    }

    #[test]
    fn liens_autorises() {
        assert!(lien_autorise("https://discord.gg/x"));
        assert!(lien_autorise("TS3SERVER://a"));
        assert!(lien_autorise("discord://x"));
        assert!(!lien_autorise("http://a"));
        assert!(!lien_autorise("file:///c:/x"));
        assert!(!lien_autorise("javascript:alert(1)"));
    }

    #[test]
    fn refuse_version_inconnue() {
        let texte = EXEMPLE.replace("\"version\": 1", "\"version\": 2");
        assert!(analyser_launcher(&texte).unwrap_err().contains("mettez a jour le launcher"));
    }

    #[test]
    fn refuse_mod_ou_plugin_douteux() {
        assert!(analyser_launcher(&EXEMPLE.replace("\"@mon_mod\"", "\"@../x\"")).is_err());
        assert!(analyser_launcher(&EXEMPLE.replace("\"@mon_mod\"", "\"mon_mod\"")).is_err());
        assert!(analyser_launcher(&EXEMPLE.replace("tfar/task_force_radio.ts3_plugin", "../x")).is_err());
        assert!(analyser_launcher(&EXEMPLE.replace("tfar/task_force_radio.ts3_plugin", "https://ailleurs/x")).is_err());
        let avec_dll = |v: &str| EXEMPLE.replace("\"tfarPlugin\"", &format!("\"tfarDll\": \"{v}\", \"tfarPlugin\""));
        assert_eq!(analyser_launcher(&avec_dll(&"a".repeat(64))).unwrap().tfar_dll, "a".repeat(64));
        assert!(analyser_launcher(&avec_dll("zz")).is_err());
    }

    #[test]
    fn etat_serveur_facultatif_et_relatif() {
        assert_eq!(analyser_launcher(EXEMPLE).unwrap().etat_serveur, "");
        let avec = |v: &str| EXEMPLE.replace("\"tfarPlugin\"", &format!("\"etatServeur\": \"{v}\", \"tfarPlugin\""));
        assert_eq!(analyser_launcher(&avec("api/serveur/etat")).unwrap().etat_serveur, "api/serveur/etat");
        assert_eq!(analyser_launcher(&avec("etat.json?v=1")).unwrap().etat_serveur, "etat.json?v=1");
        assert!(analyser_launcher(&avec("../etat")).is_err());
        assert!(analyser_launcher(&avec("https://ailleurs/etat")).is_err());
    }

    #[test]
    fn lien_avec_description() {
        let texte = EXEMPLE.replace("\"icone\": \"teamspeak\"", "\"icone\": \"teamspeak\", \"description\": \"Voix\"");
        assert_eq!(analyser_launcher(&texte).unwrap().liens[0].description, "Voix");
    }

    #[test]
    fn manifeste_valide_ou_refuse() {
        let mods = vec!["@CBA_A3".to_string()];
        let bon = format!(r#"{{"genere":"t","fichiers":[{{"chemin":"@CBA_A3/a","taille":1,"sha256":"{}"}}]}}"#, "a".repeat(64));
        assert_eq!(analyser_manifeste(&bon, &mods).unwrap().fichiers.len(), 1);
        let mauvais = bon.replace("@CBA_A3/a", "@CBA_A3/../../a");
        assert!(analyser_manifeste(&mauvais, &mods).is_err());
        assert!(analyser_manifeste("pas du json", &mods).is_err());
    }
}
