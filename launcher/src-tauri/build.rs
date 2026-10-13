// Lit launcher.config.json (racine du depot) et l'expose au code par des variables
// de compilation : env!("LAUNCHER_URL_SERVEUR"), env!("LAUNCHER_NOM"), etc.
// C'est le seul endroit ou le code Rust apprend le nom et l'adresse du serveur.
use std::path::PathBuf;

fn texte(config: &serde_json::Value, cle: &str) -> String {
    match config.get(cle).and_then(|v| v.as_str()) {
        Some(v) if !v.trim().is_empty() => v.trim().to_string(),
        _ => panic!("launcher.config.json : la cle \"{cle}\" est absente ou vide"),
    }
}

fn main() {
    let manifeste = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let chemin = std::env::var_os("LAUNCHER_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|| manifeste.join("..").join("..").join("launcher.config.json"));
    println!("cargo:rerun-if-changed={}", chemin.display());
    println!("cargo:rerun-if-env-changed=LAUNCHER_CONFIG");
    let contenu = std::fs::read_to_string(&chemin).unwrap_or_else(|e| panic!("{} illisible : {e}", chemin.display()));
    let config: serde_json::Value = serde_json::from_str(&contenu).unwrap_or_else(|e| panic!("{} : JSON invalide : {e}", chemin.display()));

    let url = texte(&config, "urlServeur");
    if !(url.starts_with("https://") || url.starts_with("http://")) || url.ends_with('/') {
        panic!("launcher.config.json : urlServeur doit etre une URL complete sans / final, recu {url}");
    }
    let dossier = texte(&config, "dossierDonnees");
    if dossier.chars().any(|c| "<>:\"/\\|?*".contains(c)) {
        panic!("launcher.config.json : dossierDonnees contient un caractere interdit sous Windows");
    }
    println!("cargo:rustc-env=LAUNCHER_URL_SERVEUR={url}");
    println!("cargo:rustc-env=LAUNCHER_NOM={}", texte(&config, "nom"));
    println!("cargo:rustc-env=LAUNCHER_DOSSIER={dossier}");
    println!("cargo:rustc-env=LAUNCHER_USER_AGENT={}", texte(&config, "userAgent"));

    tauri_build::build()
}
