use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Fichier {
    pub chemin: String,
    pub taille: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifeste {
    pub genere: String,
    pub fichiers: Vec<Fichier>,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Delta {
    pub a_telecharger: Vec<Fichier>,
    pub a_supprimer: Vec<String>,
}

impl Delta {
    pub fn vide(&self) -> bool {
        self.a_telecharger.is_empty() && self.a_supprimer.is_empty()
    }

    pub fn octets(&self) -> u64 {
        self.a_telecharger.iter().map(|f| f.taille).sum()
    }
}

pub fn chemin_valide(chemin: &str, mods: &[String]) -> bool {
    if chemin.is_empty() || chemin.starts_with('/') || chemin.contains('\\') || chemin.contains(':') {
        return false;
    }
    let segments: Vec<&str> = chemin.split('/').collect();
    if segments.len() < 2 || segments.iter().any(|s| s.is_empty() || *s == "." || *s == "..") {
        return false;
    }
    mods.iter().any(|m| m == segments[0])
}

pub fn valider(m: &Manifeste, mods: &[String]) -> Result<(), String> {
    for f in &m.fichiers {
        if !chemin_valide(&f.chemin, mods) {
            return Err(format!("Manifeste refuse, chemin invalide : {}", f.chemin));
        }
        if f.sha256.len() != 64 || !f.sha256.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(format!("Manifeste refuse, sha256 invalide : {}", f.chemin));
        }
    }
    Ok(())
}

pub fn calculer_delta<H: FnMut(&str) -> Option<String>>(
    m: &Manifeste,
    locaux: &HashMap<String, u64>,
    mods: &[String],
    mut hacher: H,
) -> Delta {
    let par_nom: HashMap<String, (&String, u64)> = locaux.iter().map(|(k, v)| (k.to_lowercase(), (k, *v))).collect();
    let attendus: HashSet<String> = m.fichiers.iter().map(|f| f.chemin.to_lowercase()).collect();
    let mut delta = Delta::default();
    for f in &m.fichiers {
        let a_jour = match par_nom.get(&f.chemin.to_lowercase()) {
            Some((reel, taille)) if *taille == f.taille => hacher(reel).is_some_and(|h| h.eq_ignore_ascii_case(&f.sha256)),
            _ => false,
        };
        if !a_jour {
            delta.a_telecharger.push(f.clone());
        }
    }
    let mut supprimer: Vec<String> = locaux
        .keys()
        .filter(|k| {
            let mod_local = k.split('/').next().unwrap_or("");
            mods.iter().any(|m| m.eq_ignore_ascii_case(mod_local)) && !attendus.contains(&k.to_lowercase())
        })
        .cloned()
        .collect();
    supprimer.sort();
    delta.a_supprimer = supprimer;
    delta
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn mods() -> Vec<String> {
        vec!["@CBA_A3".into(), "@mon_mod".into()]
    }

    fn f(chemin: &str, taille: u64, sha: char) -> Fichier {
        Fichier { chemin: chemin.into(), taille, sha256: sha.to_string().repeat(64) }
    }

    #[test]
    fn chemins_valides_et_refuses() {
        assert!(chemin_valide("@CBA_A3/addons/a.pbo", &mods()));
        for c in ["", "/@CBA_A3/a", "@CBA_A3", "@CBA_A3/../x", "@CBA_A3//a", "@CBA_A3\\a", "C:/a", "@autre/a", "@CBA_A3/./a"] {
            assert!(!chemin_valide(c, &mods()), "{c}");
        }
    }

    #[test]
    fn valider_refuse_chemin_ou_hash_invalide() {
        let bon = Manifeste { genere: "t".into(), fichiers: vec![f("@CBA_A3/a", 1, 'a')] };
        assert!(valider(&bon, &mods()).is_ok());
        let chemin = Manifeste { genere: "t".into(), fichiers: vec![f("@CBA_A3/../a", 1, 'a')] };
        assert!(valider(&chemin, &mods()).unwrap_err().contains("@CBA_A3/../a"));
        let hash = Manifeste { genere: "t".into(), fichiers: vec![f("@CBA_A3/a", 1, 'z')] };
        assert!(valider(&hash, &mods()).is_err());
    }

    #[test]
    fn delta_detecte_manquant_taille_et_hash() {
        let m = Manifeste {
            genere: "t".into(),
            fichiers: vec![f("@CBA_A3/ok", 3, 'a'), f("@CBA_A3/manquant", 3, 'a'), f("@CBA_A3/taille", 3, 'a'), f("@CBA_A3/hash", 3, 'a')],
        };
        let locaux: HashMap<String, u64> = [("@CBA_A3/ok", 3), ("@CBA_A3/taille", 4), ("@CBA_A3/hash", 3)]
            .into_iter().map(|(k, v)| (k.to_string(), v)).collect();
        let mut hashes = 0;
        let d = calculer_delta(&m, &locaux, &mods(), |c| {
            hashes += 1;
            Some(if c == "@CBA_A3/hash" { "b".repeat(64) } else { "a".repeat(64) })
        });
        let noms: Vec<&str> = d.a_telecharger.iter().map(|x| x.chemin.as_str()).collect();
        assert_eq!(noms, vec!["@CBA_A3/manquant", "@CBA_A3/taille", "@CBA_A3/hash"]);
        assert_eq!(hashes, 2);
        assert_eq!(d.octets(), 9);
        assert!(!d.vide());
    }

    #[test]
    fn delta_supprime_les_fichiers_en_trop_des_mods_geres_seulement() {
        let m = Manifeste { genere: "t".into(), fichiers: vec![f("@CBA_A3/a", 1, 'a')] };
        let locaux: HashMap<String, u64> = [("@CBA_A3/a", 1), ("@CBA_A3/vieux", 1), ("@perso/b", 1)]
            .into_iter().map(|(k, v)| (k.to_string(), v)).collect();
        let d = calculer_delta(&m, &locaux, &mods(), |_| Some("a".repeat(64)));
        assert!(d.a_telecharger.is_empty());
        assert_eq!(d.a_supprimer, vec!["@CBA_A3/vieux".to_string()]);
    }

    #[test]
    fn delta_ignore_la_casse_et_le_hash_en_majuscules() {
        let m = Manifeste { genere: "t".into(), fichiers: vec![f("@CBA_A3/Addons/A.pbo", 1, 'a')] };
        let locaux: HashMap<String, u64> = [("@cba_a3/addons/a.pbo".to_string(), 1)].into_iter().collect();
        let d = calculer_delta(&m, &locaux, &mods(), |_| Some("A".repeat(64)));
        assert!(d.vide());
    }
}
