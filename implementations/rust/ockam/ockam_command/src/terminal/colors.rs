use colorful::{core::color_string::CString, Colorful, RGB};
use colors_transform::{Color, Rgb};
use r3bl_tui::ColorWheel;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum OckamColor {
    OckamBlue,
    HeaderGradient,
    PrimaryResource,
    Success,
    Failure,
    FmtOKBackground,
    FmtINFOBackground,
    FmtWARNBackground,
    FmtERRORBackground,
    FmtLISTBackground,
}

impl OckamColor {
    pub fn value(&self) -> &str {
        match self {
            OckamColor::OckamBlue => "#52c7ea",
            OckamColor::HeaderGradient => "#4FDAB8",
            OckamColor::PrimaryResource => "#4FDAB8",
            OckamColor::Success => "#A8C97D",
            OckamColor::Failure => "#ff0000",
            OckamColor::FmtOKBackground => "#A8C97D",
            OckamColor::FmtINFOBackground => "#0DCAF0",
            OckamColor::FmtWARNBackground => "#ff9a00",
            OckamColor::FmtERRORBackground => "#ff0000",
            OckamColor::FmtLISTBackground => "#0DCAF0",
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

#[macro_export]
macro_rules! color {
    ($text:expr, $color:expr) => {
        $text.to_string().color($color.color())
    };
}

pub fn color_primary(input: impl Into<String>) -> CString {
    input.into().color(OckamColor::PrimaryResource.color())
}

pub fn color_email(input: impl Into<String>) -> CString {
    input.into().color(OckamColor::PrimaryResource.color())
}

pub fn color_uri(input: &str) -> String {
    ColorWheel::lolcat_into_string(input)
}
