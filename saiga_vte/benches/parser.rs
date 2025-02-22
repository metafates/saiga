use std::time::Duration;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use pprof::criterion::{Output, PProfProfiler};

#[derive(Default)]
struct NopPerformer {}

impl saiga_vte::Perform for NopPerformer {
    fn print(&mut self, _c: char) {}

    fn execute(&mut self, _byte: u8) {}

    fn put(&mut self, _byte: u8) {}

    fn hook(
        &mut self,
        params: &saiga_vte::params::Params,
        intermediates: &[u8],
        ignore: bool,
        action: char,
    ) {
    }

    fn unhook(&mut self) {}

    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {}

    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {}

    fn csi_dispatch(
        &mut self,
        params: &saiga_vte::params::Params,
        intermediates: &[u8],
        ignore: bool,
        action: char,
    ) {
    }
}

impl vte::Perform for NopPerformer {
    fn print(&mut self, _c: char) {}

    fn execute(&mut self, _byte: u8) {}

    fn put(&mut self, _byte: u8) {}

    fn hook(&mut self, _params: &vte::Params, _intermediates: &[u8], _ignore: bool, _action: char) {
    }

    fn unhook(&mut self) {}

    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {}

    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {}

    fn csi_dispatch(
        &mut self,
        _params: &vte::Params,
        _intermediates: &[u8],
        _ignore: bool,
        _action: char,
    ) {
    }
}

const BAT_OUTPUT: &[u8] = include_bytes!("bat.ansi");
const BIG_UTF8: &[u8] = include_bytes!("utf8.ansi");

fn alacritty_vte(c: &mut Criterion) {
    let mut parser = vte::Parser::new();
    let mut performer = NopPerformer::default();

    let mut group = c.benchmark_group("alacritty parser advance");

    group.bench_function("batch", |b| {
        b.iter(|| {
            parser.advance(&mut performer, black_box(BAT_OUTPUT));
        });
    });
    group.bench_function("batch utf8", |b| {
        b.iter(|| {
            parser.advance(&mut performer, black_box(BIG_UTF8));
        });
    });
    //
    // group.bench_function("chunks", |b| {
    //     b.iter(|| {
    //         for chunk in BAT_OUTPUT.chunks(256) {
    //             parser.advance(&mut performer, black_box(chunk));
    //         }
    //     });
    // });
    // group.bench_function("chunks utf8", |b| {
    //     b.iter(|| {
    //         for chunk in BIG_UTF8.chunks(256) {
    //             parser.advance(&mut performer, black_box(chunk));
    //         }
    //     });
    // });
    //
    // group.bench_function("sequential", |b| {
    //     b.iter(|| {
    //         for &byte in BAT_OUTPUT {
    //             parser.advance(&mut performer, black_box(&[byte]));
    //         }
    //     })
    // });
    // group.bench_function("sequential utf8", |b| {
    //     b.iter(|| {
    //         for &byte in BIG_UTF8 {
    //             parser.advance(&mut performer, black_box(&[byte]));
    //         }
    //     })
    // });

    group.finish()
}

fn parser_advance(c: &mut Criterion) {
    let mut parser = saiga_vte::Parser::new();
    let mut performer = NopPerformer::default();

    let mut group = c.benchmark_group("saiga parser advance");

    group.bench_function("batch", |b| {
        b.iter(|| {
            parser.advance(&mut performer, black_box(BAT_OUTPUT));
        });
    });

    group.bench_function("batch utf8", |b| {
        b.iter(|| {
            parser.advance(&mut performer, black_box(BIG_UTF8));
        });
    });
    //
    // group.bench_function("chunks", |b| {
    //     b.iter(|| {
    //         for chunk in BAT_OUTPUT.chunks(256) {
    //             parser.advance(&mut performer, black_box(chunk));
    //         }
    //     });
    // });
    //
    // group.bench_function("chunks utf8", |b| {
    //     b.iter(|| {
    //         for chunk in BIG_UTF8.chunks(256) {
    //             parser.advance(&mut performer, black_box(chunk));
    //         }
    //     });
    // });
    //
    // group.bench_function("sequential", |b| {
    //     b.iter(|| {
    //         for &byte in BAT_OUTPUT {
    //             parser.advance(&mut performer, black_box(&[byte]));
    //         }
    //     })
    // });
    //
    // group.bench_function("sequential utf8", |b| {
    //     b.iter(|| {
    //         for &byte in BIG_UTF8 {
    //             parser.advance(&mut performer, black_box(&[byte]));
    //         }
    //     })
    // });

    group.finish()
}

criterion_group! {
    name = benches;
    config = Criterion::default().measurement_time(Duration::from_secs(10)).with_profiler(PProfProfiler::new(50_000, Output::Flamegraph(None)));
    targets = parser_advance, alacritty_vte
}

criterion_main!(benches);
