use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use jpeg_visualizer::section::mpeg_visualization::{mpeg1::MPEG1, ts::TSDemuxer};

fn mpeg1_decode_benchmark(c: &mut Criterion) {
    let csgo = include_bytes!("../www/public/preset_videos/csgo.ts").to_vec();
    let demuxed = TSDemuxer::from_raw_bytes(csgo).parse_packets();
    c.bench_function("mpeg1 decode", |b| {
        b.iter_batched_ref(
            || MPEG1::from_bytes(demuxed.clone()),
            |mpeg1| {
                for _ in 0..300 {
                    mpeg1.decode();
                }
            },
            BatchSize::SmallInput,
        )
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = mpeg1_decode_benchmark
}
criterion_main!(benches);
