use crate::hash;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Etat {
    Absent,
    Perime,
    Ok,
}

/// DLL 64 bits installee par le plugin TeamSpeak, selon la version de TFAR publiee (`tfarVersion`).
/// TFAR stable 0.9.x : task_force_radio_win64.dll ; TFAR 1.0 beta : TFAR_win64.dll.
/// Version absente (ancien launcher.json) ou inconnue : l'une ou l'autre.
pub fn dlls(version: &str) -> &'static [&'static str] {
    match version {
        "stable" => &["task_force_radio_win64.dll"],
        "beta" => &["TFAR_win64.dll"],
        _ => &["TFAR_win64.dll", "task_force_radio_win64.dll"],
    }
}

pub fn etat_dans(appdata: &Path, version: &str, attendu: &str) -> Etat {
    let plugins = appdata.join("TS3Client").join("plugins");
    let presentes: Vec<_> = dlls(version).iter().map(|n| plugins.join(n)).filter(|d| d.is_file()).collect();
    if presentes.is_empty() {
        return Etat::Absent;
    }
    if attendu.is_empty() {
        return Etat::Ok;
    }
    let a_jour = presentes.iter().any(|dll| matches!(hash::sha256_fichier(dll), Ok(sha) if sha.eq_ignore_ascii_case(attendu)));
    if a_jour {
        Etat::Ok
    } else {
        Etat::Perime
    }
}

pub fn etat(version: &str, attendu: &str) -> Etat {
    std::env::var_os("APPDATA").map_or(Etat::Absent, |a| etat_dans(Path::new(&a), version, attendu))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ABC: &str = "BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD";

    fn poser(dir: &Path, nom: &str, contenu: &[u8]) {
        let plugins = dir.join("TS3Client").join("plugins");
        std::fs::create_dir_all(&plugins).unwrap();
        std::fs::write(plugins.join(nom), contenu).unwrap();
    }

    #[test]
    fn absent_perime_ou_a_jour() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(etat_dans(dir.path(), "", ""), Etat::Absent);
        poser(dir.path(), "TFAR_win64.dll", b"abc");
        assert_eq!(etat_dans(dir.path(), "", ""), Etat::Ok);
        assert_eq!(etat_dans(dir.path(), "", ABC), Etat::Ok);
        assert_eq!(etat_dans(dir.path(), "", &"0".repeat(64)), Etat::Perime);
    }

    #[test]
    fn la_version_choisit_la_dll() {
        let dir = tempfile::tempdir().unwrap();
        poser(dir.path(), "task_force_radio_win64.dll", b"abc");
        assert_eq!(etat_dans(dir.path(), "stable", ABC), Etat::Ok);
        assert_eq!(etat_dans(dir.path(), "beta", ABC), Etat::Absent, "un plugin 0.9 ne remplace pas la 1.0");
        assert_eq!(etat_dans(dir.path(), "", ABC), Etat::Ok, "ancien launcher.json sans tfarVersion");

        poser(dir.path(), "TFAR_win64.dll", b"autre");
        assert_eq!(etat_dans(dir.path(), "beta", ABC), Etat::Perime);
        assert_eq!(etat_dans(dir.path(), "stable", ABC), Etat::Ok, "une 1.0 restee installee ne gene pas la stable");
        assert_eq!(etat_dans(dir.path(), "", ABC), Etat::Ok);
    }
}
