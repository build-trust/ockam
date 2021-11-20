use std::fs::File;
use std::io::{BufRead, BufReader, Read};

fn print_file(file: &str) {
    let examples = std::env::var("EXAMPLES_DIR")
        .expect("set EXAMPLES_DIR environment variable to source location");
    let file = format!("{}/{}", examples, file);

    let err = format!("Can't find example source {}", file);
    let mut file = File::open(file).unwrap_or_else(|_| panic!("{}", err));

    let mut content = String::new();
    file.read_to_string(&mut content).expect("short read");
    println!("{}", content);
}

fn main() {
    let file: String = std::env::args().nth(1).expect("missing file argument");

    let file = File::open(file).expect("unable to open file");
    let file = BufReader::new(file);

    let mut in_example = false;

    for line in file.lines() {
        match line {
            Ok(line) => {
                let example_start = line.starts_with("// examples/");
                let block_end = line.starts_with("```");

                if in_example {
                    if block_end {
                        in_example = false;
                        println!("{}", line);
                    }
                } else {
                    println!("{}", line);
                    if example_start {
                        in_example = true;
                        let example_name = line.split('/').last().expect("no example filename");
                        print_file(example_name);
                    }
                }
            }
            _ => break,
        }
    }
}
