use colorful::RGB;
use colors_transform::{Color, Rgb};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum OckamColor {
    OckamBlue,
    HeaderGradient,
    PrimaryResource,
    FmtTextColor,
    FmtOKBackground,
    FmtINFOBackground,
    FmtWARNBackground,
    FmtERRORBackground,
}

impl OckamColor {
    pub fn value(&self) -> &str {
        match self {
            OckamColor::OckamBlue => "#52c7ea",
            OckamColor::HeaderGradient => "#4FDAB8",
            OckamColor::PrimaryResource => "#4FDAB8",
            OckamColor::FmtTextColor => "#0A1A2B",
            OckamColor::FmtOKBackground => "#61d800",
            OckamColor::FmtINFOBackground => "#0DCAF0",
            OckamColor::FmtWARNBackground => "#ff9a00",
            OckamColor::FmtERRORBackground => "#ff0000",
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
