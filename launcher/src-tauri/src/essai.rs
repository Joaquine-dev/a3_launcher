//! Essai de bout en bout contre un vrai serveur de mises a jour (docker compose de serveur/).
//! Meme enchainement que les commandes `verifier` puis `mettre_a_jour` du launcher :
//! launcher.json, manifest.json, delta local, synchronisation, nouveau delta (qui doit etre vide).
//!
//! Ignore par defaut. A lancer a la main, serveur demarre et publie :
//!
//!   ESSAI_SERVEUR=http://127.0.0.1:8080 ESSAI_MODS=C:\chemin\mods cargo test --lib essai -- --ignored --nocapture
//!
//! Chaque passage affiche les fichiers telecharges et supprimes : modifier un fichier cote serveur
//! et republier, puis relancer, doit donner exactement un telechargement.
use crate::{distant, hash, manifest, sync};
use std::path::PathBuf;

async fn texte(client: &reqwest::Client, base: &str, chemin: &str) -> String {
    let url = format!("{base}/{chemin}");
    client.get(&url).send().await.and_then(|r| r.error_for_status()).unwrap_or_else(|e| panic!("{url} : {e}")).text().await.unwrap()
}

async fn delta(client: &reqwest::Client, base: &str, racine: &std::path::Path) -> (Vec<String>, manifest::Delta) {
    let launcher = distant::analyser_launcher(&texte(client, base, "launcher.json").await).unwrap();
    let manifeste = distant::analyser_manifeste(&texte(client, base, "manifest.json").await, &launcher.mods).unwrap();
    sync::nettoyer_parts(racine, &launcher.mods);
    let locaux = sync::lister_locaux(racine, &launcher.mods);
    let mut cache = hash::CacheHash::default();
    let d = manifest::calculer_delta(&manifeste, &locaux, &launcher.mods, |c| cache.hash(racine, c));
    (launcher.mods, d)
}

#[tokio::test]
#[ignore = "demande un serveur local : ESSAI_SERVEUR et ESSAI_MODS"]
async fn essai_serveur_local() {
    let base = std::env::var("ESSAI_SERVEUR").expect("ESSAI_SERVEUR=http://127.0.0.1:8080");
    let racine = PathBuf::from(std::env::var("ESSAI_MODS").expect("ESSAI_MODS=dossier des mods local"));
    std::fs::create_dir_all(&racine).unwrap();
    let client = reqwest::Client::builder().user_agent("Arma-Launcher-essai").build().unwrap();

    let (mods, avant) = delta(&client, &base, &racine).await;
    println!("mods : {}", mods.join(", "));
    for f in &avant.a_telecharger {
        println!("  a telecharger : {} ({} octets)", f.chemin, f.taille);
    }
    for c in &avant.a_supprimer {
        println!("  a supprimer   : {c}");
    }
    sync::synchroniser(&client, &base, &racine, &avant, |_| {}).await.unwrap();

    let (_, apres) = delta(&client, &base, &racine).await;
    assert!(apres.vide(), "delta non vide apres synchronisation : {apres:?}");
    for f in &avant.a_telecharger {
        let local = racine.join(&f.chemin);
        assert_eq!(hash::sha256_fichier(&local).unwrap(), f.sha256, "{}", f.chemin);
    }
    for c in &avant.a_supprimer {
        assert!(!racine.join(c).exists(), "{c} aurait du etre supprime");
    }
    println!("RESUME telecharges={} supprimes={} octets={}", avant.a_telecharger.len(), avant.a_supprimer.len(), avant.octets());
}
