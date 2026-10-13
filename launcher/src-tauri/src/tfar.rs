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

pub fn etat_dans(appdata: &Path, attendu: &str) -> Etat {
    let dll = appdata.join("TS3Client").join("plugins").join("TFAR_win64.dll");
    if !dll.is_file() {
        return Etat::Absent;
    }
    if attendu.is_empty() {
        return Etat::Ok;
    }
    match hash::sha256_fichier(&dll) {
        Ok(sha) if sha.eq_ignore_ascii_case(attendu) => Etat::Ok,
        _ => Etat::Perime,
    }
}

pub fn etat(attendu: &str) -> Etat {
    std::env::var_os("APPDATA").map_or(Etat::Absent, |a| etat_dans(Path::new(&a), attendu))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absent_perime_ou_a_jour() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(etat_dans(dir.path(), ""), Etat::Absent);
        let plugins = dir.path().join("TS3Client").join("plugins");
        std::fs::create_dir_all(&plugins).unwrap();
        std::fs::write(plugins.join("TFAR_win64.dll"), b"abc").unwrap();
        assert_eq!(etat_dans(dir.path(), ""), Etat::Ok);
        let bon = "BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD";
        assert_eq!(etat_dans(dir.path(), bon), Etat::Ok);
        assert_eq!(etat_dans(dir.path(), &"0".repeat(64)), Etat::Perime);
    }
}
