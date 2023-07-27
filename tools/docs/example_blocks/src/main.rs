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

    let file = File::open(file.clone()).unwrap_or_else(|_| panic!("unable to open file {file}"));
    let file = BufReader::new(file);

    let mut in_example = false;

    for line in file.lines() {
        match line {
            Ok(line) => {
                let example_start = line.starts_with("// ") && line.ends_with(".rs");
                let block_end = line.eq("```");

                if example_start {
                    println!("{}", line);

                    let example_name = line.strip_prefix("// ").expect("no example filename");
                    in_example = true;
                    print_file(example_name);
                } else {
                    if block_end {
                        in_example = false;
                    } else if in_example {
                        continue;
                    }

                    println!("{}", line);
                }
            }
            _ => break,
        }
    }
}
