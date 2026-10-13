use crate::distant::Lien;
use std::path::{Path, PathBuf};

/// Marge gardee libre sur le disque en plus du telechargement.
const RESERVE: u64 = 200 * 1024 * 1024;

pub fn teamspeak_installe_dans(appdata: Option<&Path>, programmes: &[PathBuf]) -> bool {
    appdata.is_some_and(|a| a.join("TS3Client").is_dir()) || client_teamspeak_dans(programmes).is_some()
}

pub fn client_teamspeak_dans(programmes: &[PathBuf]) -> Option<PathBuf> {
    programmes.iter().map(|p| p.join("TeamSpeak 3 Client").join("ts3client_win64.exe")).find(|c| c.is_file())
}

fn programmes() -> Vec<PathBuf> {
    let mut programmes: Vec<PathBuf> = ["ProgramFiles", "ProgramFiles(x86)"].iter().filter_map(|v| std::env::var_os(v).map(PathBuf::from)).collect();
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        programmes.push(PathBuf::from(local).join("Programs"));
    }
    programmes
}

pub fn teamspeak_installe() -> bool {
    let appdata = std::env::var_os("APPDATA").map(PathBuf::from);
    teamspeak_installe_dans(appdata.as_deref(), &programmes())
}

/// Client TeamSpeak 3, lance directement : l'association ts3server:// de Windows peut pointer vers un TeamSpeak 5/6 desinstalle.
pub fn client_teamspeak() -> Option<PathBuf> {
    client_teamspeak_dans(&programmes())
}

/// Programme de la commande `"C:\...\x.exe" --arg "%1"` ou `C:\...\x.exe "%1"`.
pub fn programme_de_commande(commande: &str) -> Option<PathBuf> {
    let c = commande.trim();
    let chemin = match c.strip_prefix('"') {
        Some(reste) => reste.split('"').next()?,
        None => c.split_whitespace().next()?,
    };
    (!chemin.is_empty()).then(|| PathBuf::from(chemin))
}

/// Vrai si Windows sait ouvrir ts3server:// avec un programme qui existe encore.
#[cfg(windows)]
pub fn association_ts3server_valide() -> bool {
    use winreg::enums::HKEY_CLASSES_ROOT;
    use winreg::RegKey;
    RegKey::predef(HKEY_CLASSES_ROOT)
        .open_subkey(r"ts3server\shell\open\command")
        .and_then(|c| c.get_value::<String, _>(""))
        .ok()
        .and_then(|c| programme_de_commande(&c))
        .is_some_and(|p| p.is_file())
}

#[cfg(not(windows))]
pub fn association_ts3server_valide() -> bool {
    true
}

/// Lance par le shell Windows : un programme marque administrateur affiche l'UAC au lieu de l'erreur 740.
#[cfg(windows)]
pub fn lancer_par_le_shell(programme: &Path, argument: &str) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    let large = |s: &std::ffi::OsStr| s.encode_wide().chain(Some(0)).collect::<Vec<u16>>();
    let verbe = large("open".as_ref());
    let fichier = large(programme.as_os_str());
    let parametres = large(format!("\"{}\"", argument.replace('"', "")).as_ref());
    let resultat = unsafe { ShellExecuteW(std::ptr::null_mut(), verbe.as_ptr(), fichier.as_ptr(), parametres.as_ptr(), std::ptr::null(), SW_SHOWNORMAL) };
    if resultat as isize > 32 {
        Ok(())
    } else {
        Err(format!("code {}", resultat as isize))
    }
}

#[cfg(not(windows))]
pub fn lancer_par_le_shell(programme: &Path, argument: &str) -> Result<(), String> {
    std::process::Command::new(programme).arg(argument).spawn().map(|_| ()).map_err(|x| x.to_string())
}

pub fn lien_teamspeak(liens: &[Lien]) -> Option<&str> {
    liens.iter().map(|l| l.url.as_str()).find(|u| u.to_ascii_lowercase().starts_with("ts3server://"))
}

/// Octets libres sur le volume du dossier, en remontant au premier parent qui existe. None si illisible.
pub fn espace_libre(dossier: &Path) -> Option<u64> {
    let existant = dossier.ancestors().find(|p| p.is_dir())?;
    fs2::available_space(existant).ok()
}

/// Octets qui manquent pour telecharger `a_telecharger` en gardant la reserve ; 0 = assez de place ou volume illisible.
pub fn espace_manquant(libre: Option<u64>, a_telecharger: u64) -> u64 {
    match libre {
        Some(l) if a_telecharger > 0 => (a_telecharger + RESERVE).saturating_sub(l),
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn teamspeak_par_le_profil_ou_par_le_programme() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!teamspeak_installe_dans(Some(dir.path()), &[dir.path().to_path_buf()]));
        std::fs::create_dir_all(dir.path().join("TS3Client")).unwrap();
        assert!(teamspeak_installe_dans(Some(dir.path()), &[]));

        let prog = tempfile::tempdir().unwrap();
        let client = prog.path().join("TeamSpeak 3 Client");
        std::fs::create_dir_all(&client).unwrap();
        std::fs::write(client.join("ts3client_win64.exe"), b"").unwrap();
        assert!(teamspeak_installe_dans(None, &[prog.path().to_path_buf()]));
        assert_eq!(client_teamspeak_dans(&[dir.path().to_path_buf(), prog.path().to_path_buf()]), Some(client.join("ts3client_win64.exe")));
    }

    #[test]
    fn lit_le_programme_d_une_commande_d_association() {
        assert_eq!(programme_de_commande(r#""C:\T S\TeamSpeak.exe" --connect="%1""#), Some(PathBuf::from(r"C:\T S\TeamSpeak.exe")));
        assert_eq!(programme_de_commande(r#"C:\TS\ts3client_win64.exe "%1""#), Some(PathBuf::from(r"C:\TS\ts3client_win64.exe")));
        assert_eq!(programme_de_commande("  "), None);
    }

    #[test]
    fn trouve_le_lien_teamspeak() {
        let lien = |url: &str| Lien { nom: "x".into(), url: url.into(), icone: String::new(), description: String::new() };
        let liens = vec![lien("https://discord.gg/abc"), lien("TS3server://play.example.com?port=9987")];
        assert_eq!(lien_teamspeak(&liens), Some("TS3server://play.example.com?port=9987"));
        assert_eq!(lien_teamspeak(&liens[..1]), None);
    }

    #[test]
    fn espace_libre_remonte_au_parent_existant() {
        let dir = tempfile::tempdir().unwrap();
        assert!(espace_libre(&dir.path().join("pas").join("encore").join("cree")).is_some());
    }

    #[test]
    fn manque_calcule_avec_la_reserve() {
        let go = 1024 * 1024 * 1024;
        assert_eq!(espace_manquant(Some(10 * go), go), 0);
        assert_eq!(espace_manquant(Some(go), go), RESERVE);
        assert_eq!(espace_manquant(Some(0), 0), 0);
        assert_eq!(espace_manquant(None, go), 0);
    }
}
