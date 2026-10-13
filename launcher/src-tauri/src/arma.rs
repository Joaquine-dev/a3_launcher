use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct Lancement<'a> {
    pub dossier_mods: &'a Path,
    pub mods: &'a [String],
    pub parametres_serveur: &'a str,
    pub parametres_joueur: &'a str,
    pub hote: &'a str,
    pub port: u16,
    pub mot_de_passe: &'a str,
}

pub fn arguments(l: &Lancement) -> Vec<String> {
    let mut args: Vec<String> = ["2", "1", "1", "-exe", "Arma3_x64.exe"].iter().map(|s| s.to_string()).collect();
    args.extend(l.parametres_serveur.split_whitespace().map(String::from));
    args.extend(l.parametres_joueur.split_whitespace().map(String::from));
    // Sans -world, Arma charge une carte en fond du menu principal : demarrage plus long.
    // Un -world= pose par le serveur (launcher.json) ou par le joueur gagne.
    if !args.iter().any(|a| a.starts_with("-world=")) {
        args.push("-world=empty".to_string());
    }
    args.push(format!("-connect={}", l.hote));
    args.push(format!("-port={}", l.port));
    if !l.mot_de_passe.is_empty() {
        args.push(format!("-password={}", l.mot_de_passe));
    }
    let mods: Vec<String> = l.mods.iter().map(|m| l.dossier_mods.join(m).to_string_lossy().into_owned()).collect();
    args.push(format!("-mod={}", mods.join(";")));
    args
}

pub fn dossier_valide(dossier: &Path) -> bool {
    dossier.join("Arma3_x64.exe").is_file() && dossier.join("Arma3BattlEye.exe").is_file()
}

#[cfg(windows)]
pub fn dossier_registre() -> Option<PathBuf> {
    use winreg::enums::HKEY_LOCAL_MACHINE;
    use winreg::RegKey;
    let cle = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(r"SOFTWARE\WOW6432Node\bohemia interactive\arma 3").ok()?;
    let main: String = cle.get_value("main").ok()?;
    let dossier = PathBuf::from(main);
    dossier_valide(&dossier).then_some(dossier)
}

#[cfg(not(windows))]
pub fn dossier_registre() -> Option<PathBuf> {
    None
}

pub fn lancer(dossier: &Path, arguments: &[String]) -> io::Result<()> {
    Command::new(dossier.join("Arma3BattlEye.exe")).args(arguments).current_dir(dossier).spawn().map(|_| ())
}

#[cfg(windows)]
pub fn processus_en_cours(image: &str) -> bool {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    Command::new("tasklist")
        .args(["/FI", &format!("IMAGENAME eq {image}"), "/NH"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_ascii_lowercase().contains(&image.to_ascii_lowercase()))
        .unwrap_or(false)
}

#[cfg(not(windows))]
pub fn processus_en_cours(_image: &str) -> bool {
    false
}

pub fn arma_en_cours() -> bool {
    processus_en_cours("arma3_x64.exe")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mods() -> Vec<String> {
        vec!["@CBA_A3".into(), "@task_force_radio".into(), "@mon_mod".into()]
    }

    #[test]
    fn ligne_de_commande_complete() {
        let dossier = PathBuf::from("C:\\Users\\Jo Doe\\AppData\\Local\\Arma Launcher\\mods");
        let m = mods();
        let args = arguments(&Lancement {
            dossier_mods: &dossier,
            mods: &m,
            parametres_serveur: "-skipIntro  -noSplash",
            parametres_joueur: "-world=empty",
            hote: "play.example.com",
            port: 2302,
            mot_de_passe: "",
        });
        assert_eq!(
            args,
            vec![
                "2", "1", "1", "-exe", "Arma3_x64.exe", "-skipIntro", "-noSplash", "-world=empty",
                "-connect=play.example.com", "-port=2302",
                "-mod=C:\\Users\\Jo Doe\\AppData\\Local\\Arma Launcher\\mods\\@CBA_A3;C:\\Users\\Jo Doe\\AppData\\Local\\Arma Launcher\\mods\\@task_force_radio;C:\\Users\\Jo Doe\\AppData\\Local\\Arma Launcher\\mods\\@mon_mod",
            ]
        );
    }

    #[test]
    fn world_empty_ajoute_une_seule_fois() {
        let dossier = PathBuf::from("D:\\m");
        let m = vec!["@CBA_A3".to_string()];
        let sans = arguments(&Lancement { dossier_mods: &dossier, mods: &m, parametres_serveur: "", parametres_joueur: "", hote: "h", port: 2302, mot_de_passe: "" });
        assert_eq!(sans.iter().filter(|a| a.starts_with("-world=")).count(), 1);
        assert!(sans.contains(&"-world=empty".to_string()));

        let pose = arguments(&Lancement { dossier_mods: &dossier, mods: &m, parametres_serveur: "", parametres_joueur: "-world=Stratis", hote: "h", port: 2302, mot_de_passe: "" });
        assert_eq!(pose.iter().filter(|a| a.starts_with("-world=")).count(), 1);
        assert!(pose.contains(&"-world=Stratis".to_string()));
    }

    #[test]
    fn mot_de_passe_ajoute_s_il_existe() {
        let dossier = PathBuf::from("D:\\m");
        let m = vec!["@CBA_A3".to_string()];
        let args = arguments(&Lancement { dossier_mods: &dossier, mods: &m, parametres_serveur: "", parametres_joueur: "", hote: "h", port: 2402, mot_de_passe: "secret" });
        assert!(args.contains(&"-password=secret".to_string()));
        assert!(args.contains(&"-port=2402".to_string()));
    }

    #[test]
    fn dossier_valide_exige_les_deux_executables() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!dossier_valide(dir.path()));
        std::fs::write(dir.path().join("Arma3_x64.exe"), b"").unwrap();
        assert!(!dossier_valide(dir.path()));
        std::fs::write(dir.path().join("Arma3BattlEye.exe"), b"").unwrap();
        assert!(dossier_valide(dir.path()));
    }
}
