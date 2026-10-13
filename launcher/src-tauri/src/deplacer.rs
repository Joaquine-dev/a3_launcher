use std::fs;
use std::io;
use std::path::Path;

/// Meme dossier, sans tenir compte de la casse ni du separateur final (Windows).
pub fn meme_dossier(a: &Path, b: &Path) -> bool {
    let norme = |p: &Path| p.to_string_lossy().replace('/', "\\").trim_end_matches('\\').to_lowercase();
    norme(a) == norme(b)
}

/// Mods a deplacer : presents dans l'ancien dossier, absents du nouveau.
pub fn a_deplacer(ancien: &Path, nouveau: &Path, mods: &[String]) -> Vec<String> {
    mods.iter().filter(|m| ancien.join(m).is_dir() && !nouveau.join(m).exists()).cloned().collect()
}

pub fn taille_dossier(dossier: &Path) -> u64 {
    fs::read_dir(dossier)
        .map(|entrees| {
            entrees
                .flatten()
                .map(|e| match e.file_type() {
                    Ok(t) if t.is_dir() => taille_dossier(&e.path()),
                    _ => e.metadata().map(|m| m.len()).unwrap_or(0),
                })
                .sum()
        })
        .unwrap_or(0)
}

fn copier_dossier(source: &Path, cible: &Path, fait: &mut u64, progression: &mut dyn FnMut(u64)) -> io::Result<()> {
    fs::create_dir_all(cible)?;
    for entree in fs::read_dir(source)? {
        let entree = entree?;
        let chemin = entree.path();
        let destination = cible.join(entree.file_name());
        if entree.file_type()?.is_dir() {
            copier_dossier(&chemin, &destination, fait, progression)?;
        } else {
            let meta = entree.metadata()?;
            fs::copy(&chemin, &destination)?;
            if let Ok(date) = meta.modified() {
                let _ = fs::File::options().write(true).open(&destination).and_then(|f| f.set_modified(date));
            }
            *fait += meta.len();
            progression(*fait);
        }
    }
    Ok(())
}

/// Deplace les dossiers `mods` de `ancien` vers `nouveau` : renommage sur le meme volume, copie puis suppression sinon.
/// En cas d'echec, ce qui a deja bouge revient a sa place et l'erreur remonte. L'ancien dossier est retire s'il est vide.
pub fn deplacer(ancien: &Path, nouveau: &Path, mods: &[String], libre: Option<u64>, progression: &mut dyn FnMut(u64, u64)) -> Result<(), String> {
    if meme_dossier(ancien, nouveau) {
        return Ok(());
    }
    if mods.iter().any(|m| nouveau.starts_with(ancien.join(m))) {
        return Err("Le nouveau dossier des mods ne peut pas etre a l'interieur d'un mod".into());
    }
    let liste = a_deplacer(ancien, nouveau, mods);
    if liste.is_empty() {
        return Ok(());
    }
    fs::create_dir_all(nouveau).map_err(|x| format!("Creation du dossier impossible : {x}"))?;

    let mut renommes: Vec<String> = Vec::new();
    let mut a_copier: Vec<String> = Vec::new();
    for m in &liste {
        match fs::rename(ancien.join(m), nouveau.join(m)) {
            Ok(()) => renommes.push(m.clone()),
            Err(_) => a_copier.push(m.clone()),
        }
    }
    let annuler = |renommes: &[String]| {
        for m in renommes {
            let _ = fs::rename(nouveau.join(m), ancien.join(m));
        }
    };

    if !a_copier.is_empty() {
        let total: u64 = a_copier.iter().map(|m| taille_dossier(&ancien.join(m))).sum();
        if libre.is_some_and(|l| l < total) {
            annuler(&renommes);
            return Err(format!("Pas assez de place dans le nouveau dossier ({} Mo necessaires)", total / 1024 / 1024));
        }
        let mut fait = 0u64;
        let mut copies: Vec<String> = Vec::new();
        for m in &a_copier {
            if let Err(x) = copier_dossier(&ancien.join(m), &nouveau.join(m), &mut fait, &mut |f| progression(f, total)) {
                let _ = fs::remove_dir_all(nouveau.join(m));
                for c in &copies {
                    let _ = fs::remove_dir_all(nouveau.join(c));
                }
                annuler(&renommes);
                return Err(format!("Deplacement de {m} impossible : {x}"));
            }
            copies.push(m.clone());
        }
        for m in &a_copier {
            let _ = fs::remove_dir_all(ancien.join(m));
        }
    }
    let _ = fs::remove_dir(ancien);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mods() -> Vec<String> {
        vec!["@a".into(), "@b".into()]
    }

    fn remplir(racine: &Path) {
        for m in ["@a", "@b"] {
            fs::create_dir_all(racine.join(m).join("addons")).unwrap();
            fs::write(racine.join(m).join("addons").join("x.pbo"), m.as_bytes()).unwrap();
        }
    }

    #[test]
    fn compare_les_dossiers_sans_la_casse() {
        assert!(meme_dossier(Path::new(r"C:\Mods\Serveur\"), Path::new(r"c:\mods\serveur")));
        assert!(!meme_dossier(Path::new(r"C:\Mods\Serveur"), Path::new(r"D:\Mods\Serveur")));
    }

    #[test]
    fn deplace_les_mods_et_retire_l_ancien_dossier_vide() {
        let dir = tempfile::tempdir().unwrap();
        let ancien = dir.path().join("ancien");
        let nouveau = dir.path().join("nouveau");
        remplir(&ancien);
        deplacer(&ancien, &nouveau, &mods(), None, &mut |_, _| {}).unwrap();
        assert_eq!(fs::read(nouveau.join("@b").join("addons").join("x.pbo")).unwrap(), b"@b");
        assert!(!ancien.exists());
    }

    #[test]
    fn laisse_ce_qui_n_est_pas_un_mod_et_ce_qui_existe_deja() {
        let dir = tempfile::tempdir().unwrap();
        let ancien = dir.path().join("ancien");
        let nouveau = dir.path().join("nouveau");
        remplir(&ancien);
        fs::write(ancien.join("perso.txt"), b"x").unwrap();
        fs::create_dir_all(nouveau.join("@a")).unwrap();
        deplacer(&ancien, &nouveau, &mods(), None, &mut |_, _| {}).unwrap();
        assert!(ancien.join("@a").is_dir());
        assert!(!ancien.join("@b").exists());
        assert!(nouveau.join("@b").is_dir());
        assert!(ancien.join("perso.txt").is_file());
    }

    #[test]
    fn refuse_un_dossier_dans_un_mod() {
        let dir = tempfile::tempdir().unwrap();
        remplir(dir.path());
        assert!(deplacer(dir.path(), &dir.path().join("@a").join("sous"), &mods(), None, &mut |_, _| {}).is_err());
        assert!(dir.path().join("@a").join("addons").join("x.pbo").is_file());
    }

    #[test]
    fn la_copie_garde_contenu_date_et_compte_les_octets() {
        let dir = tempfile::tempdir().unwrap();
        remplir(&dir.path().join("src"));
        let source = dir.path().join("src").join("@a");
        let cible = dir.path().join("dst").join("@a");
        let mut fait = 0;
        let mut vus = Vec::new();
        copier_dossier(&source, &cible, &mut fait, &mut |f| vus.push(f)).unwrap();
        let x = |p: &Path| fs::metadata(p.join("addons").join("x.pbo")).unwrap().modified().unwrap();
        assert_eq!(x(&source), x(&cible));
        assert_eq!(fait, 2);
        assert_eq!(vus, vec![2]);
        assert_eq!(taille_dossier(&source), 2);
    }
}
