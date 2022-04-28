use std::time::Instant;

const SAMPLE: &str = "/ip4/127.0.0.1/tcp/80/ip4/192.168.0.1/ip6/::1/tcp/443/dnsaddr/example.com/tcp/80";

#[test]
fn benchmark() {
    bench("multiaddr/read-string: ", || multiaddr::Multiaddr::try_from(SAMPLE).is_ok());
    bench("ockam_multiaddr/read-string: ", || ockam_multiaddr::MultiAddr::try_from(SAMPLE).is_ok());

    let ma = multiaddr::Multiaddr::try_from(SAMPLE).unwrap();

    bench("multiaddr/read-bytes: ", || { multiaddr::Multiaddr::from_iter(ma.into_iter()); true });
    bench("ockam_multiaddr/read-bytes: ", || ockam_multiaddr::MultiAddr::try_from(ma.as_ref()).is_ok());
}

fn bench(label: &str, f: impl Fn() -> bool) {
    const ROUNDS: u32 = 1000;
    let start = Instant::now();
    for _ in 0 .. ROUNDS {
        assert!(f())
    }
    eprintln!("{label} {:0.2?}", start.elapsed() / ROUNDS)
}
