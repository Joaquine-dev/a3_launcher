use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::Path;
use std::time::UNIX_EPOCH;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntreeCache {
    pub taille: u64,
    pub modifie: u64,
    pub sha256: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CacheHash {
    pub entrees: HashMap<String, EntreeCache>,
}

pub fn sha256_fichier(chemin: &Path) -> io::Result<String> {
    let mut fichier = File::open(chemin)?;
    let mut hasher = Sha256::new();
    let mut tampon = vec![0u8; 1024 * 1024];
    loop {
        let n = fichier.read(&mut tampon)?;
        if n == 0 {
            break;
        }
        hasher.update(&tampon[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

impl CacheHash {
    pub fn charger(chemin: &Path) -> CacheHash {
        fs::read_to_string(chemin).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
    }

    pub fn sauver(&self, chemin: &Path) -> io::Result<()> {
        if let Some(parent) = chemin.parent() {
            fs::create_dir_all(parent)?;
        }
        let tmp = chemin.with_extension("json.tmp");
        fs::write(&tmp, serde_json::to_vec(self).map_err(io::Error::other)?)?;
        fs::rename(tmp, chemin)
    }

    pub fn hash(&mut self, racine: &Path, chemin: &str) -> Option<String> {
        let plein = racine.join(chemin);
        let meta = fs::metadata(&plein).ok()?;
        let modifie = meta.modified().ok()?.duration_since(UNIX_EPOCH).ok()?.as_millis() as u64;
        if let Some(e) = self.entrees.get(chemin) {
            if e.taille == meta.len() && e.modifie == modifie {
                return Some(e.sha256.clone());
            }
        }
        let sha256 = sha256_fichier(&plein).ok()?;
        self.entrees.insert(chemin.to_string(), EntreeCache { taille: meta.len(), modifie, sha256: sha256.clone() });
        Some(sha256)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    const ABC: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    #[test]
    fn sha256_connu() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("a.txt");
        fs::write(&p, b"abc").unwrap();
        assert_eq!(sha256_fichier(&p).unwrap(), ABC);
    }

    #[test]
    fn cache_reutilise_puis_recalcule_si_la_taille_change() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("@m")).unwrap();
        fs::write(dir.path().join("@m/a.txt"), b"abc").unwrap();
        let mut cache = CacheHash::default();
        assert_eq!(cache.hash(dir.path(), "@m/a.txt").as_deref(), Some(ABC));
        cache.entrees.get_mut("@m/a.txt").unwrap().sha256 = "memorise".into();
        assert_eq!(cache.hash(dir.path(), "@m/a.txt").as_deref(), Some("memorise"));
        fs::write(dir.path().join("@m/a.txt"), b"abcd").unwrap();
        assert_ne!(cache.hash(dir.path(), "@m/a.txt").as_deref(), Some("memorise"));
    }

    #[test]
    fn fichier_absent_donne_none() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(CacheHash::default().hash(dir.path(), "@m/rien"), None);
    }

    #[test]
    fn sauver_puis_charger() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("@m")).unwrap();
        fs::write(dir.path().join("@m/a.txt"), b"abc").unwrap();
        let mut cache = CacheHash::default();
        cache.hash(dir.path(), "@m/a.txt");
        let fichier = dir.path().join("cache").join("hash-cache.json");
        cache.sauver(&fichier).unwrap();
        let relu = CacheHash::charger(&fichier);
        assert_eq!(relu.entrees.get("@m/a.txt").unwrap().sha256, ABC);
        assert!(CacheHash::charger(&dir.path().join("absent.json")).entrees.is_empty());
    }
}
