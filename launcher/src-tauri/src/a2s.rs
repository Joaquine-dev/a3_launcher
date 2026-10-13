use serde::Serialize;
use std::time::{Duration, Instant};
use tokio::net::UdpSocket;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct InfoServeur {
    pub nom: String,
    pub carte: String,
    pub joueurs: u8,
    pub max: u8,
    pub ping: u32,
}

#[derive(Debug, PartialEq)]
pub enum Reponse {
    Challenge([u8; 4]),
    Info(InfoServeur),
}

pub fn requete(challenge: Option<[u8; 4]>) -> Vec<u8> {
    let mut v = vec![0xFF, 0xFF, 0xFF, 0xFF, 0x54];
    v.extend_from_slice(b"Source Engine Query\0");
    if let Some(c) = challenge {
        v.extend_from_slice(&c);
    }
    v
}

fn chaine(octets: &[u8], position: &mut usize) -> Result<String, String> {
    let debut = *position;
    let longueur = octets.get(debut..).ok_or("A2S : paquet tronque")?.iter().position(|&b| b == 0).ok_or("A2S : chaine non terminee")?;
    *position = debut + longueur + 1;
    Ok(String::from_utf8_lossy(&octets[debut..debut + longueur]).into_owned())
}

pub fn decoder(octets: &[u8]) -> Result<Reponse, String> {
    if octets.len() < 5 || octets[..4] != [0xFF; 4] {
        return Err("A2S : en-tete invalide".into());
    }
    match octets[4] {
        0x41 if octets.len() >= 9 => Ok(Reponse::Challenge([octets[5], octets[6], octets[7], octets[8]])),
        0x49 => {
            let mut position = 6;
            let nom = chaine(octets, &mut position)?;
            let carte = chaine(octets, &mut position)?;
            chaine(octets, &mut position)?;
            chaine(octets, &mut position)?;
            position += 2;
            let joueurs = *octets.get(position).ok_or("A2S : paquet tronque")?;
            let max = *octets.get(position + 1).ok_or("A2S : paquet tronque")?;
            Ok(Reponse::Info(InfoServeur { nom, carte, joueurs, max, ping: 0 }))
        }
        autre => Err(format!("A2S : type inattendu 0x{autre:02X}")),
    }
}

pub async fn interroger(hote: &str, port_query: u16, delai: Duration) -> Option<InfoServeur> {
    let socket = UdpSocket::bind("0.0.0.0:0").await.ok()?;
    socket.connect((hote, port_query)).await.ok()?;
    let mut challenge = None;
    for _ in 0..3 {
        let envoi = Instant::now();
        socket.send(&requete(challenge)).await.ok()?;
        let mut tampon = [0u8; 1400];
        let n = tokio::time::timeout(delai, socket.recv(&mut tampon)).await.ok()?.ok()?;
        let aller_retour = envoi.elapsed().as_millis().min(u32::MAX as u128) as u32;
        match decoder(&tampon[..n]).ok()? {
            Reponse::Challenge(c) => challenge = Some(c),
            Reponse::Info(info) => return Some(InfoServeur { ping: aller_retour, ..info }),
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reponse_info() -> Vec<u8> {
        let mut v = vec![0xFF, 0xFF, 0xFF, 0xFF, 0x49, 0x11];
        for s in ["Mon Serveur", "Stratis", "Arma3", "Arma 3"] {
            v.extend_from_slice(s.as_bytes());
            v.push(0);
        }
        v.extend_from_slice(&[0x00, 0x00, 12, 32, 0, b'd', b'w', 0, 1]);
        v
    }

    #[test]
    fn requete_sans_et_avec_challenge() {
        let mut attendu = vec![0xFF, 0xFF, 0xFF, 0xFF, 0x54];
        attendu.extend_from_slice(b"Source Engine Query\0");
        assert_eq!(requete(None), attendu);
        attendu.extend_from_slice(&[1, 2, 3, 4]);
        assert_eq!(requete(Some([1, 2, 3, 4])), attendu);
    }

    #[test]
    fn decode_challenge_et_info() {
        assert_eq!(decoder(&[0xFF, 0xFF, 0xFF, 0xFF, 0x41, 9, 8, 7, 6]).unwrap(), Reponse::Challenge([9, 8, 7, 6]));
        assert_eq!(
            decoder(&reponse_info()).unwrap(),
            Reponse::Info(InfoServeur { nom: "Mon Serveur".into(), carte: "Stratis".into(), joueurs: 12, max: 32, ping: 0 })
        );
    }

    #[test]
    fn refuse_les_paquets_invalides() {
        assert!(decoder(&[0x00, 0x01]).is_err());
        assert!(decoder(&[0xFF, 0xFF, 0xFF, 0xFF, 0x49, 0x11, b'x']).is_err());
        assert!(decoder(&[0xFF, 0xFF, 0xFF, 0xFF, 0x42]).is_err());
    }

    #[tokio::test]
    async fn interroge_un_serveur_qui_exige_un_challenge() {
        let socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let port = socket.local_addr().unwrap().port();
        tokio::spawn(async move {
            let mut tampon = [0u8; 1400];
            loop {
                let (n, de) = socket.recv_from(&mut tampon).await.unwrap();
                if n == 25 {
                    socket.send_to(&[0xFF, 0xFF, 0xFF, 0xFF, 0x41, 1, 2, 3, 4], de).await.unwrap();
                } else if tampon[25..29] == [1, 2, 3, 4] {
                    socket.send_to(&reponse_info(), de).await.unwrap();
                }
            }
        });
        let info = interroger("127.0.0.1", port, Duration::from_secs(2)).await.unwrap();
        assert_eq!((info.joueurs, info.max), (12, 32));
    }

    #[tokio::test]
    async fn serveur_muet_donne_none() {
        let muet = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
        let port = muet.local_addr().unwrap().port();
        assert_eq!(interroger("127.0.0.1", port, Duration::from_millis(300)).await, None);
    }

    #[tokio::test]
    #[ignore]
    async fn serveur_reel() {
        println!("{:?}", interroger("play.example.com", 2303, Duration::from_secs(3)).await);
    }
}
