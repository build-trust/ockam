use ockamd::cli::Args;
use ockamd::node::Node;

fn main() {
    let args = Args::parse();
    let config = args.into();

    match Node::new(&config) {
        Ok(node) => node.run(),
        Err(s) => {
            println!("Failed to create node: {}", s);
        }
    }
}
