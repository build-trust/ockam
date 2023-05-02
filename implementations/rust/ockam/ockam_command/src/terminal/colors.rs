use colorful::RGB;
use colors_transform::{Color, Rgb};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum OckamColor {
    OckamBlue,
    PrimaryGradient,
    DeepBlue,
    SuccessGreen,
}

impl OckamColor {
    pub fn value(&self) -> &str {
        match self {
            OckamColor::OckamBlue => "#52c7ea",
            OckamColor::PrimaryGradient => "#4FDAB8",
            OckamColor::DeepBlue => "#0A1A2B",
            OckamColor::SuccessGreen => "#85ff00",
        }
    }

    pub fn color(&self) -> colorful::RGB {
        let rgb = Rgb::from_hex_str(self.value()).expect("Invalid hex string");

        RGB::new(
            rgb.get_red() as u8,
            rgb.get_green() as u8,
            rgb.get_blue() as u8,
        )
    }
}
