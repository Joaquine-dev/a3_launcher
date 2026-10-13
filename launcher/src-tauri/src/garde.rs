use serde::{Deserialize, Serialize};

/// Secondes avant un redemarrage en dessous desquelles JOUER est retenu.
pub const MARGE_AVANT: i64 = 120;
/// Secondes apres l'heure prevue pendant lesquelles on tient encore le redemarrage pour en cours.
pub const MARGE_APRES: i64 = 60;

#[derive(Debug, Clone, Deserialize)]
pub struct EtatSite {
    #[serde(rename = "enLigne")]
    pub en_ligne: bool,
    #[serde(rename = "rebootDans", default)]
    pub reboot_dans: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "raison", rename_all = "snake_case")]
pub enum Garde {
    Libre,
    Redemarrage { secondes: i64 },
    HorsLigne,
}

pub fn analyser(texte: &str) -> Option<EtatSite> {
    serde_json::from_str(texte).ok()
}

/// Le site injoignable ne retient jamais JOUER : seul un fait etabli le fait.
/// Un serveur qui ne repond pas a la requete Steam n'est tenu pour eteint que si le site le dit aussi,
/// un pare-feu qui coupe l'UDP du joueur ne doit pas l'empecher de jouer.
pub fn garde(a2s_repond: bool, site: Option<&EtatSite>) -> Garde {
    if let Some(s) = site.and_then(|s| s.reboot_dans) {
        if (-MARGE_APRES..=MARGE_AVANT).contains(&s) {
            return Garde::Redemarrage { secondes: s.max(0) };
        }
    }
    if !a2s_repond && site.is_some_and(|s| !s.en_ligne) {
        return Garde::HorsLigne;
    }
    Garde::Libre
}

#[cfg(test)]
mod tests {
    use super::*;

    fn site(en_ligne: bool, reboot_dans: Option<i64>) -> EtatSite {
        EtatSite { en_ligne, reboot_dans }
    }

    #[test]
    fn serveur_en_ligne_loin_du_redemarrage() {
        assert_eq!(garde(true, Some(&site(true, Some(3600)))), Garde::Libre);
        assert_eq!(garde(true, Some(&site(true, None))), Garde::Libre);
    }

    #[test]
    fn retenu_dans_les_deux_minutes_avant_le_redemarrage() {
        assert_eq!(garde(true, Some(&site(true, Some(121)))), Garde::Libre);
        assert_eq!(garde(true, Some(&site(true, Some(120)))), Garde::Redemarrage { secondes: 120 });
        assert_eq!(garde(true, Some(&site(true, Some(5)))), Garde::Redemarrage { secondes: 5 });
    }

    #[test]
    fn retenu_pendant_la_minute_qui_suit_l_heure_prevue() {
        assert_eq!(garde(true, Some(&site(true, Some(-30)))), Garde::Redemarrage { secondes: 0 });
        assert_eq!(garde(false, Some(&site(false, Some(-61)))), Garde::HorsLigne);
    }

    #[test]
    fn un_vieux_battement_ne_retient_rien() {
        assert_eq!(garde(true, Some(&site(true, Some(-7200)))), Garde::Libre);
    }

    #[test]
    fn hors_ligne_seulement_si_le_site_le_confirme() {
        assert_eq!(garde(false, Some(&site(false, None))), Garde::HorsLigne);
        assert_eq!(garde(false, Some(&site(true, None))), Garde::Libre);
        assert_eq!(garde(false, None), Garde::Libre);
    }

    #[test]
    fn lit_la_reponse_du_site() {
        let s = analyser(r#"{"enLigne":true,"joueurs":3,"rebootLe":"2026-09-21T01:00:00+02:00","version":"1.0.0","vuLe":null,"rebootDans":90}"#).unwrap();
        assert!(s.en_ligne);
        assert_eq!(s.reboot_dans, Some(90));
        let ancien = analyser(r#"{"enLigne":false,"joueurs":0}"#).unwrap();
        assert_eq!(ancien.reboot_dans, None);
        assert!(analyser("<html>").is_none());
    }
}
