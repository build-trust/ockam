#[cfg(feature = "ui")]
pub mod colors;

#[cfg(not(feature = "ui"))]
pub mod colors {
    use colorful::{core::color_string::CString, Colorful, RGB};
    use std::fmt::Display;

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
        /// Hex string value for this color (kept consistent w/ full UI feature version).
        pub fn value(&self) -> &'static str {
            match self {
                OckamColor::OckamBlue => "#52c7ea",
                OckamColor::HeaderGradient => "#4FDAB8",
                OckamColor::PrimaryResource => "#4FDAB8",
                OckamColor::Success => "#A8C97D",
                OckamColor::Failure => "#ff0000",
                OckamColor::FmtOKBackground => "#A8C97D",
                OckamColor::FmtINFOBackground => "#0DCAF0",
                OckamColor::FmtWARNBackground => "#ff9a00",
                OckamColor::FmtERRORBackground => "#FF0000",
                OckamColor::FmtLISTBackground => "#0DCAF0",
            }
        }
        pub fn color(&self) -> RGB {
            // Provide simple, hard-coded fallback colors that don't require r3bl_* crates.
            match self {
                OckamColor::OckamBlue => RGB::new(0x52, 0xC7, 0xEA),
                OckamColor::HeaderGradient => RGB::new(0x4F, 0xDA, 0xB8),
                OckamColor::PrimaryResource => RGB::new(0x4F, 0xDA, 0xB8),
                OckamColor::Success => RGB::new(0xA8, 0xC9, 0x7D),
                OckamColor::Failure => RGB::new(0xFF, 0x00, 0x00),
                OckamColor::FmtOKBackground => RGB::new(0xA8, 0xC9, 0x7D),
                OckamColor::FmtINFOBackground => RGB::new(0x0D, 0xCA, 0xF0),
                OckamColor::FmtWARNBackground => RGB::new(0xFF, 0x9A, 0x00),
                OckamColor::FmtERRORBackground => RGB::new(0xFF, 0x00, 0x00),
                OckamColor::FmtLISTBackground => RGB::new(0x0D, 0xCA, 0xF0),
            }
        }
    }

    // Re-exported macro to keep parity w/ full UI feature.
    #[macro_export]
    macro_rules! color {
        ($text:expr, $color:expr) => {
            $text.to_string().color($color.color())
        };
    }

    pub fn color_primary(input: impl Display) -> CString {
        input.to_string().color(OckamColor::PrimaryResource.color())
    }
    pub fn color_primary_alt(input: impl Display) -> String {
        // Fallback: no gradient, just plain text
        input.to_string()
    }
    pub fn color_ok(input: impl Display) -> CString {
        input.to_string().color(OckamColor::FmtOKBackground.color())
    }
    pub fn color_warn(input: impl Display) -> CString {
        input.to_string().color(OckamColor::FmtWARNBackground.color())
    }
    pub fn color_error(input: impl Display) -> CString {
        input.to_string().color(OckamColor::FmtERRORBackground.color())
    }
    pub fn color_email(input: impl Display) -> CString {
        input.to_string().color(OckamColor::PrimaryResource.color())
    }
    pub fn color_uri(input: impl Display) -> String {
        // Fallback: no gradient
        input.to_string()
    }
}

pub mod command;
pub mod output;
pub mod terminal;
