use criterion::{black_box, criterion_group, criterion_main, Criterion};

criterion_group!(benches, benchmark);
criterion_main!(benches);

const SAMPLE: &str = "/ip4/127.0.0.1/tcp/80/ip4/192.168.0.1/ip6/::1/tcp/443/dns/example.com/tcp/80";

fn benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse-string");
    let ma = multiaddr::Multiaddr::try_from(SAMPLE).unwrap();
    group.bench_function("multiaddr", |b| {
        b.iter(|| black_box(multiaddr::Multiaddr::try_from(SAMPLE)))
    });
    group.bench_function("ockam-multiaddr", |b| {
        b.iter(|| black_box(ockam_multiaddr::MultiAddr::try_from(SAMPLE)))
    });
    group.finish();
    let mut group = c.benchmark_group("parse-bytes");
    group.bench_function("multiaddr", |b| {
        b.iter(|| black_box(multiaddr::Multiaddr::from_iter(ma.into_iter())))
    });
    group.bench_function("ockam-multiaddr", |b| {
        b.iter(|| black_box(ockam_multiaddr::MultiAddr::try_from(ma.as_ref())))
    });
    group.finish();
}
