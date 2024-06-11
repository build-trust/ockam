/// There are two available formats, plain text and JSON,
/// which are handled by the Terminal struct.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputFormat {
    Plain,
    Json {
        pretty: bool,
        jq_query: Option<String>,
    },
}

impl OutputFormat {
    pub fn is_plain(&self) -> bool {
        matches!(self, Self::Plain)
    }

    pub fn is_json(&self) -> bool {
        matches!(self, Self::Json { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_format() {
        let plain = OutputFormat::Plain;
        assert!(plain.is_plain());
        assert!(!plain.is_json());

        let json = OutputFormat::Json {
            pretty: false,
            jq_query: None,
        };
        assert!(json.is_json());
        assert!(!json.is_plain());
    }
}
