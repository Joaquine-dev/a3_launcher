use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApresLancement {
    Reduire,
    Fermer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Reglages {
    pub dossier_arma: Option<PathBuf>,
    pub dossier_mods: PathBuf,
    pub parametres: String,
    pub apres_lancement: ApresLancement,
    pub rejoindre_teamspeak: bool,
}

fn variable(nom: &str) -> PathBuf {
    std::env::var_os(nom).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."))
}

pub fn dossier_appli() -> PathBuf {
    variable("APPDATA").join(env!("LAUNCHER_DOSSIER"))
}

impl Default for Reglages {
    fn default() -> Self {
        Reglages {
            dossier_arma: None,
            dossier_mods: variable("LOCALAPPDATA").join(env!("LAUNCHER_DOSSIER")).join("mods"),
            parametres: String::new(),
            apres_lancement: ApresLancement::Reduire,
            rejoindre_teamspeak: true,
        }
    }
}

pub fn charger(chemin: &Path) -> Reglages {
    fs::read_to_string(chemin).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

pub fn sauver(chemin: &Path, r: &Reglages) -> io::Result<()> {
    if let Some(parent) = chemin.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = chemin.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_vec_pretty(r).map_err(io::Error::other)?)?;
    fs::rename(tmp, chemin)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fichier_absent_donne_les_defauts() {
        let dir = tempfile::tempdir().unwrap();
        let r = charger(&dir.path().join("absent.json"));
        assert_eq!(r.dossier_arma, None);
        assert!(r.dossier_mods.ends_with("mods"));
        assert_eq!(r.apres_lancement, ApresLancement::Reduire);
    }

    #[test]
    fn sauver_puis_charger() {
        let dir = tempfile::tempdir().unwrap();
        let chemin = dir.path().join("sous").join("config.json");
        let r = Reglages {
            dossier_arma: Some(PathBuf::from("C:\\Jeux\\Arma 3")),
            dossier_mods: PathBuf::from("D:\\Mods Serveur"),
            parametres: "-world=empty".into(),
            apres_lancement: ApresLancement::Fermer,
            rejoindre_teamspeak: false,
        };
        sauver(&chemin, &r).unwrap();
        let relu = charger(&chemin);
        assert_eq!(relu.dossier_arma, r.dossier_arma);
        assert_eq!(relu.dossier_mods, r.dossier_mods);
        assert_eq!(relu.parametres, "-world=empty");
        assert_eq!(relu.apres_lancement, ApresLancement::Fermer);
        assert!(!relu.rejoindre_teamspeak);
    }

    #[test]
    fn champs_manquants_completes() {
        let dir = tempfile::tempdir().unwrap();
        let chemin = dir.path().join("config.json");
        std::fs::write(&chemin, r#"{"parametres":"-noSplash"}"#).unwrap();
        let r = charger(&chemin);
        assert_eq!(r.parametres, "-noSplash");
        assert!(r.dossier_mods.ends_with("mods"));
        assert!(r.rejoindre_teamspeak);
    }
}
