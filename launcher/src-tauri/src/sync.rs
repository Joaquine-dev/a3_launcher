use crate::manifest::{Delta, Fichier};
use futures_util::{stream, StreamExt};
use percent_encoding::{utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::io::AsyncWriteExt;

const SEGMENT: &AsciiSet = &NON_ALPHANUMERIC.remove(b'-').remove(b'_').remove(b'.').remove(b'~').remove(b'@');
const PARALLELE: usize = 4;
const ESSAIS: usize = 3;

#[derive(Debug, Clone, Serialize)]
pub struct Progression {
    pub fait: u64,
    pub total: u64,
}

pub fn url_fichier(base: &str, chemin: &str) -> String {
    let segments: Vec<String> = chemin.split('/').map(|s| utf8_percent_encode(s, SEGMENT).to_string()).collect();
    format!("{}/mods/{}", base.trim_end_matches('/'), segments.join("/"))
}

fn parcourir(dossier: &Path, racine: &Path, sortie: &mut Vec<(PathBuf, String, u64)>) {
    let Ok(entrees) = fs::read_dir(dossier) else { return };
    for entree in entrees.flatten() {
        let chemin = entree.path();
        let Ok(meta) = entree.metadata() else { continue };
        if meta.is_dir() {
            parcourir(&chemin, racine, sortie);
        } else if let Ok(relatif) = chemin.strip_prefix(racine) {
            let relatif = relatif.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/");
            sortie.push((chemin, relatif, meta.len()));
        }
    }
}

fn fichiers_des_mods(racine: &Path, mods: &[String]) -> Vec<(PathBuf, String, u64)> {
    let mut sortie = Vec::new();
    for m in mods {
        parcourir(&racine.join(m), racine, &mut sortie);
    }
    sortie
}

pub fn lister_locaux(racine: &Path, mods: &[String]) -> HashMap<String, u64> {
    fichiers_des_mods(racine, mods).into_iter().filter(|(_, r, _)| !r.ends_with(".part")).map(|(_, r, t)| (r, t)).collect()
}

pub fn nettoyer_parts(racine: &Path, mods: &[String]) {
    for (plein, relatif, _) in fichiers_des_mods(racine, mods) {
        if relatif.ends_with(".part") {
            let _ = fs::remove_file(plein);
        }
    }
}

async fn essai(client: &reqwest::Client, url: &str, part: &Path, fait: &AtomicU64) -> Result<(u64, String), (u64, String)> {
    let reponse = client.get(url).send().await.and_then(|r| r.error_for_status()).map_err(|e| (0, e.to_string()))?;
    let mut fichier = tokio::fs::File::create(part).await.map_err(|e| (0, e.to_string()))?;
    let mut hasher = Sha256::new();
    let mut lu = 0u64;
    let mut flux = reponse.bytes_stream();
    while let Some(morceau) = flux.next().await {
        let morceau = match morceau {
            Ok(m) => m,
            Err(e) => return Err((lu, e.to_string())),
        };
        hasher.update(&morceau);
        if let Err(e) = fichier.write_all(&morceau).await {
            return Err((lu, e.to_string()));
        }
        lu += morceau.len() as u64;
        fait.fetch_add(morceau.len() as u64, Ordering::Relaxed);
    }
    if let Err(e) = fichier.flush().await {
        return Err((lu, e.to_string()));
    }
    Ok((lu, format!("{:x}", hasher.finalize())))
}

async fn telecharger(client: &reqwest::Client, base: &str, racine: &Path, f: &Fichier, fait: &AtomicU64) -> Result<(), String> {
    let destination = racine.join(&f.chemin);
    let part = PathBuf::from(format!("{}.part", destination.to_string_lossy()));
    if let Some(parent) = destination.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|e| format!("{} : {}", f.chemin, e))?;
    }
    let url = url_fichier(base, &f.chemin);
    let mut derniere = String::new();
    for _ in 0..ESSAIS {
        let resultat = essai(client, &url, &part, fait).await;
        match resultat {
            Ok((taille, sha)) if taille == f.taille && sha.eq_ignore_ascii_case(&f.sha256) => {
                return match tokio::fs::rename(&part, &destination).await {
                    Ok(()) => Ok(()),
                    Err(e) => {
                        let _ = tokio::fs::remove_file(&part).await;
                        Err(format!("{} : {}", f.chemin, e))
                    }
                };
            }
            Ok((taille, _)) => {
                fait.fetch_sub(taille, Ordering::Relaxed);
                derniere = format!("{} : contenu different du manifeste", f.chemin);
            }
            Err((taille, e)) => {
                fait.fetch_sub(taille, Ordering::Relaxed);
                derniere = format!("{} : {}", f.chemin, e);
            }
        }
        let _ = tokio::fs::remove_file(&part).await;
    }
    Err(derniere)
}

pub async fn synchroniser<F: Fn(Progression) + Send + Sync>(
    client: &reqwest::Client,
    base: &str,
    racine: &Path,
    delta: &Delta,
    rapport: F,
) -> Result<(), String> {
    for chemin in &delta.a_supprimer {
        if let Err(e) = tokio::fs::remove_file(racine.join(chemin)).await {
            if e.kind() != std::io::ErrorKind::NotFound {
                return Err(format!("{} : {}", chemin, e));
            }
        }
    }
    let total = delta.octets();
    let fait = AtomicU64::new(0);
    let telechargements = async {
        let taches: Vec<_> = delta.a_telecharger.iter().map(|f| telecharger(client, base, racine, f, &fait)).collect();
        let mut flux = stream::iter(taches).buffer_unordered(PARALLELE);
        while let Some(resultat) = flux.next().await {
            resultat?;
        }
        Ok::<(), String>(())
    };
    tokio::pin!(telechargements);
    let mut intervalle = tokio::time::interval(Duration::from_millis(250));
    let resultat = loop {
        tokio::select! {
            r = &mut telechargements => break r,
            _ = intervalle.tick() => rapport(Progression { fait: fait.load(Ordering::Relaxed), total }),
        }
    };
    if resultat.is_err() {
        for f in &delta.a_telecharger {
            let part = PathBuf::from(format!("{}.part", racine.join(&f.chemin).to_string_lossy()));
            let _ = tokio::fs::remove_file(part).await;
        }
    }
    rapport(Progression { fait: fait.load(Ordering::Relaxed), total });
    resultat
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{Delta, Fichier};
    use sha2::{Digest, Sha256};
    use std::collections::HashMap;
    use std::fs;
    use std::sync::{Arc, Mutex};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    async fn serveur(fichiers: HashMap<String, Vec<u8>>) -> String {
        let ecoute = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let adresse = ecoute.local_addr().unwrap();
        tokio::spawn(async move {
            while let Ok((mut flux, _)) = ecoute.accept().await {
                let fichiers = fichiers.clone();
                tokio::spawn(async move {
                    let mut tampon = vec![0u8; 8192];
                    let mut lu = 0;
                    loop {
                        let n = flux.read(&mut tampon[lu..]).await.unwrap_or(0);
                        if n == 0 {
                            return;
                        }
                        lu += n;
                        if tampon[..lu].windows(4).any(|w| w == b"\r\n\r\n") {
                            break;
                        }
                    }
                    let requete = String::from_utf8_lossy(&tampon[..lu]).to_string();
                    let chemin = requete.split_whitespace().nth(1).unwrap_or("/").to_string();
                    let reponse = match fichiers.get(&chemin) {
                        Some(corps) => {
                            let mut r = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", corps.len()).into_bytes();
                            r.extend_from_slice(corps);
                            r
                        }
                        None => b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec(),
                    };
                    let _ = flux.write_all(&reponse).await;
                });
            }
        });
        format!("http://{adresse}")
    }

    fn sha(octets: &[u8]) -> String {
        format!("{:x}", Sha256::digest(octets))
    }

    fn mods() -> Vec<String> {
        vec!["@m".into()]
    }

    #[test]
    fn url_encode_les_segments() {
        assert_eq!(url_fichier("http://x", "@m/addons/a b.pbo"), "http://x/mods/@m/addons/a%20b.pbo");
    }

    #[test]
    fn liste_les_fichiers_des_mods_geres_sans_les_part() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("@m/addons")).unwrap();
        fs::create_dir_all(dir.path().join("@autre")).unwrap();
        fs::write(dir.path().join("@m/addons/a.pbo"), b"12345").unwrap();
        fs::write(dir.path().join("@m/addons/b.pbo.part"), b"1").unwrap();
        fs::write(dir.path().join("@autre/c.pbo"), b"1").unwrap();
        let locaux = lister_locaux(dir.path(), &mods());
        assert_eq!(locaux.len(), 1);
        assert_eq!(locaux.get("@m/addons/a.pbo"), Some(&5));
        nettoyer_parts(dir.path(), &mods());
        assert!(!dir.path().join("@m/addons/b.pbo.part").exists());
    }

    #[tokio::test]
    async fn telecharge_verifie_renomme_et_supprime() {
        let base = serveur([("/mods/@m/addons/a.pbo".to_string(), b"bonjour".to_vec())].into_iter().collect()).await;
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("@m")).unwrap();
        fs::write(dir.path().join("@m/vieux.txt"), b"x").unwrap();
        let delta = Delta {
            a_telecharger: vec![Fichier { chemin: "@m/addons/a.pbo".into(), taille: 7, sha256: sha(b"bonjour") }],
            a_supprimer: vec!["@m/vieux.txt".into()],
        };
        let rapports = Arc::new(Mutex::new(Vec::new()));
        let r = rapports.clone();
        synchroniser(&reqwest::Client::new(), &base, dir.path(), &delta, move |p| r.lock().unwrap().push(p)).await.unwrap();
        assert_eq!(fs::read(dir.path().join("@m/addons/a.pbo")).unwrap(), b"bonjour");
        assert!(!dir.path().join("@m/addons/a.pbo.part").exists());
        assert!(!dir.path().join("@m/vieux.txt").exists());
        let dernier = rapports.lock().unwrap().last().cloned().unwrap();
        assert_eq!((dernier.fait, dernier.total), (7, 7));
    }

    #[tokio::test]
    async fn hash_faux_echoue_sans_laisser_de_fichier() {
        let base = serveur([("/mods/@m/a.pbo".to_string(), b"corrompu".to_vec())].into_iter().collect()).await;
        let dir = tempfile::tempdir().unwrap();
        let delta = Delta {
            a_telecharger: vec![Fichier { chemin: "@m/a.pbo".into(), taille: 8, sha256: sha(b"attendu!") }],
            a_supprimer: vec![],
        };
        let erreur = synchroniser(&reqwest::Client::new(), &base, dir.path(), &delta, |_| {}).await.unwrap_err();
        assert!(erreur.contains("@m/a.pbo"), "{erreur}");
        assert!(!dir.path().join("@m/a.pbo").exists());
        assert!(!dir.path().join("@m/a.pbo.part").exists());
    }

    #[tokio::test]
    async fn fichier_absent_du_serveur_echoue() {
        let base = serveur(HashMap::new()).await;
        let dir = tempfile::tempdir().unwrap();
        let delta = Delta {
            a_telecharger: vec![Fichier { chemin: "@m/a.pbo".into(), taille: 1, sha256: sha(b"a") }],
            a_supprimer: vec![],
        };
        assert!(synchroniser(&reqwest::Client::new(), &base, dir.path(), &delta, |_| {}).await.is_err());
    }

    #[tokio::test]
    async fn suppression_impossible_retourne_une_erreur() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("@m/dossier")).unwrap();
        let delta = Delta { a_telecharger: vec![], a_supprimer: vec!["@m/dossier".into()] };
        let erreur = synchroniser(&reqwest::Client::new(), "http://127.0.0.1:1", dir.path(), &delta, |_| {}).await.unwrap_err();
        assert!(erreur.contains("@m/dossier"), "{erreur}");
        assert!(dir.path().join("@m/dossier").exists());
    }

    #[tokio::test]
    async fn telechargement_reussi_remplace_le_fichier_existant() {
        let base = serveur([("/mods/@m/addons/a.pbo".to_string(), b"nouveau".to_vec())].into_iter().collect()).await;
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("@m/addons")).unwrap();
        fs::write(dir.path().join("@m/addons/a.pbo"), b"ancien!").unwrap();
        let delta = Delta {
            a_telecharger: vec![Fichier { chemin: "@m/addons/a.pbo".into(), taille: 7, sha256: sha(b"nouveau") }],
            a_supprimer: vec![],
        };
        synchroniser(&reqwest::Client::new(), &base, dir.path(), &delta, |_| {}).await.unwrap();
        assert_eq!(fs::read(dir.path().join("@m/addons/a.pbo")).unwrap(), b"nouveau");
        assert!(!dir.path().join("@m/addons/a.pbo.part").exists());
    }

    #[tokio::test]
    async fn echec_partiel_ne_laisse_aucun_part_sur_le_disque() {
        let base = serveur([("/mods/@m/bon.pbo".to_string(), b"bonjour".to_vec())].into_iter().collect()).await;
        let dir = tempfile::tempdir().unwrap();
        let delta = Delta {
            a_telecharger: vec![
                Fichier { chemin: "@m/bon.pbo".into(), taille: 7, sha256: sha(b"bonjour") },
                Fichier { chemin: "@m/absent.pbo".into(), taille: 1, sha256: sha(b"a") },
            ],
            a_supprimer: vec![],
        };
        let erreur = synchroniser(&reqwest::Client::new(), &base, dir.path(), &delta, |_| {}).await;
        assert!(erreur.is_err());
        assert!(!dir.path().join("@m/bon.pbo.part").exists());
        assert!(!dir.path().join("@m/absent.pbo.part").exists());
    }
}
