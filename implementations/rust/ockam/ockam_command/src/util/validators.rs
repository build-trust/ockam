use miette::miette;
use regex::Regex;

pub(crate) fn cloud_resource_name_validator(s: &str) -> miette::Result<()> {
    let project_name_regex = Regex::new(r"^[a-zA-Z0-9]+([a-zA-Z0-9-_\.]?[a-zA-Z0-9])*$").unwrap();
    let is_project_name_valid = project_name_regex.is_match(s);
    if !is_project_name_valid {
        Err(miette!("Invalid name"))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_validate_cloud_resource_name() {
        let valid_names: Vec<&str> = vec![
            "name",
            "0001",
            "321_11-11-22",
            "0_0",
            "6.9",
            "0-9",
            "name_with_underscores",
            "name-with-dashes",
            "name.with.dots",
            "name1with2numbers3",
            "11name22with33numbers00",
            "76123long.name_with-underscores.and-dashes_and3dots00and.numbers",
        ];
        for name in valid_names {
            assert!(cloud_resource_name_validator(name).is_ok());
        }

        let invalid_names: Vec<&str> = vec![
            "name with spaces in between",
            " name-with-leading-space",
            "name.with.trailing.space ",
            " name-with-leading-and-trailing-space ",
            "     name_with_multiple_leading_space",
            "name__with_consecutive_underscore",
            "_name_with_leading_underscore",
            "name-with-trailing-underscore_",
            "name_with_consecutive---dashes",
            "name_with_trailing_dashes--",
            "---name_with_leading_dashes",
            "name-with-consecutive...dots",
            "name.with.trailing.dots....",
            ".name_with-leading.dot",
            "name_.with._consecutive-_-dots.-.dashes-._underscores",
            "1 2 3 4",
            "  1234",
            "_",
            "__",
            ". _ .",
        ];
        for name in invalid_names {
            assert!(cloud_resource_name_validator(name).is_err());
        }
    }
}
