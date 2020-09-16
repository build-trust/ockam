use std::io::stdin;

use ockamd::args::Args;

fn main() {
    let args = Args::parse();
    println!("args: {:?}", args);

    // read stdin for messages to encrypt and route
    let input = stdin();
    let mut buf = String::new();

    loop {
        match input.read_line(&mut buf) {
            Ok(n) => {
                if n > 0 {
                    print!("ockamd: {}", buf);
                    buf.clear();
                }
            }
            Err(e) => {
                println!("error: {:?}", e);
            }
        }
    }
}
