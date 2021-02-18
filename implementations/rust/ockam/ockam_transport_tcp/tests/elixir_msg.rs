use std::env;
use std::process;

#[test]
pub fn main() {
    let args: Vec<String> = env::args().collect();

    println!("Args: {:?}", args);
}
