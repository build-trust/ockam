use low_latency_portal::run_inlet;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    run_inlet(args.get(1).cloned(), None);
}
