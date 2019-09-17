
pub fn generate_key_pair() -> i32 {
    500
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_generates_key_pair() {
        assert_eq!(500, generate_key_pair());
    }
}
