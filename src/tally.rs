use roland_rs::devices::v160hd::TallyState;

use crate::actions::{SELECT_PGM, SELECT_PST};
use crate::settings::ActionSettings;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TallyCheck {
    Off,
    Pgm,
    Prv,
    Both,
}

impl TallyCheck {
    pub fn parse(value: &str, action: &str) -> Self {
        match value {
            "off" => Self::Off,
            "pgm" => Self::Pgm,
            "prv" => Self::Prv,
            "both" => Self::Both,
            _ if action == SELECT_PGM => Self::Pgm,
            _ if action == SELECT_PST => Self::Prv,
            _ => Self::Off,
        }
    }

    pub fn light(self, state: TallyState) -> Option<TallyLight> {
        match (self, state) {
            (Self::Off, _) | (_, TallyState::Off) => None,
            (Self::Pgm, TallyState::Program | TallyState::Both) => Some(TallyLight::Program),
            (Self::Pgm, _) => None,
            (Self::Prv, TallyState::Preview | TallyState::Both) => Some(TallyLight::Preview),
            (Self::Prv, _) => None,
            (Self::Both, TallyState::Program) => Some(TallyLight::Program),
            (Self::Both, TallyState::Preview) => Some(TallyLight::Preview),
            (Self::Both, TallyState::Both) => Some(TallyLight::Both),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TallyLight {
    Program,
    Preview,
    Both,
}

#[derive(Debug, Clone)]
pub struct TallyBinding {
    pub source: Option<u8>,
    pub check: TallyCheck,
}

impl TallyBinding {
    pub fn from_action(action: &str, settings: &ActionSettings) -> Self {
        Self {
            source: tally_source_index(&settings.source),
            check: TallyCheck::parse(&settings.tally_check, action),
        }
    }

    pub fn watches_tally(&self) -> bool {
        self.check != TallyCheck::Off && self.source.is_some()
    }
}

pub fn tally_source_index(source: &str) -> Option<u8> {
    let (kind, n) = source.split_once(':')?;
    let n: u8 = n.parse().ok()?;
    match kind {
        "hdmi" if (1..=8).contains(&n) => Some(n - 1),
        "sdi" if (1..=8).contains(&n) => Some(0x08 + n - 1),
        _ => None,
    }
}

pub fn image_data_uri(light: TallyLight) -> String {
    let svg = match light {
        TallyLight::Program => solid("#E10600"),
        TallyLight::Preview => solid("#00A651"),
        TallyLight::Both => {
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"144\" height=\"144\"><rect width=\"144\" height=\"72\" fill=\"#E10600\"/><rect y=\"72\" width=\"144\" height=\"72\" fill=\"#00A651\"/></svg>"
                .to_string()
        }
    };
    format!("data:image/svg+xml;charset=utf8,{svg}")
}

fn solid(fill: &str) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="144" height="144"><rect width="144" height="144" fill="{fill}"/></svg>"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hdmi_and_sdi_map_to_tally_index() {
        assert_eq!(tally_source_index("hdmi:1"), Some(0));
        assert_eq!(tally_source_index("sdi:1"), Some(8));
        assert_eq!(tally_source_index("still:1"), None);
    }

    #[test]
    fn both_check_lights_pgm_and_prv() {
        let check = TallyCheck::Both;
        assert_eq!(check.light(TallyState::Program), Some(TallyLight::Program));
        assert_eq!(check.light(TallyState::Preview), Some(TallyLight::Preview));
        assert_eq!(check.light(TallyState::Off), None);
    }

    #[test]
    fn off_never_lights() {
        assert_eq!(TallyCheck::Off.light(TallyState::Program), None);
        assert_eq!(TallyCheck::Off.light(TallyState::Preview), None);
    }
}
