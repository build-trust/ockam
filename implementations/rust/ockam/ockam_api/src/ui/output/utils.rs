use crate::colors::OckamColor;
use crate::ConnectionStatus;
use colorful::core::color_string::CString;
use colorful::Colorful;
use ockam::identity::TimestampInSeconds;

pub fn comma_separated<T: AsRef<str>>(data: &[T]) -> String {
    if data.is_empty() {
        "-".to_string()
    } else {
        data.iter()
            .map(AsRef::as_ref)
            .collect::<Vec<_>>()
            .join(", ")
    }
}

pub fn human_readable_time(time: TimestampInSeconds) -> String {
    use time::format_description::well_known::iso8601::*;
    use time::Error::Format;
    use time::OffsetDateTime;

    match OffsetDateTime::from_unix_timestamp(*time as i64) {
        Ok(time) => {
            let config = Iso8601::<
                {
                    Config::DEFAULT
                        .set_time_precision(TimePrecision::Second {
                            decimal_digits: None,
                        })
                        .encode()
                },
            >;
            time.format(&config).unwrap_or_else(|_| {
                Format(time::error::Format::InvalidComponent("timestamp error")).to_string()
            })
        }
        Err(_) => Format(time::error::Format::InvalidComponent(
            "unix time is invalid",
        ))
        .to_string(),
    }
}

pub fn colorize_connection_status(status: ConnectionStatus) -> CString {
    let text = status.to_string();
    match status {
        ConnectionStatus::Up => text.color(OckamColor::PrimaryResource.color()),
        ConnectionStatus::Down => text.color(OckamColor::Failure.color()),
        ConnectionStatus::Degraded => text.color(OckamColor::Failure.color()),
    }
}

pub fn indent(indent: impl Into<String>, text: impl Into<String>) -> String {
    let indent: String = indent.into();
    text.into()
        .split('\n')
        .map(|line| format!("{indent}{line}"))
        .collect::<Vec<String>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comma_separated() {
        let data = vec!["a", "b", "c"];
        let result = comma_separated(&data);
        assert_eq!(result, "a, b, c");
    }

    #[test]
    fn test_indent() {
        let result = indent("---", "line1\nthen line2\n and finally line3");
        assert_eq!(result, "---line1\n---then line2\n--- and finally line3");
    }
}
