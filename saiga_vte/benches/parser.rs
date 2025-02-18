use std::time::Duration;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use vte::{Params, Perform};

#[derive(Default)]
struct NopPerformer {}

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

const INPUT: &[u8] = include_bytes!("bench.ansi");

fn alacritty_vte(c: &mut Criterion) {
    let mut parser = vte::Parser::new();
    let mut performer = NopPerformer::default();

    let mut group = c.benchmark_group("alacritty parser advance");

    group.bench_function("batch", |b| {
        b.iter(|| {
            parser.advance(&mut performer, black_box(INPUT));
        });
    });

    group.bench_function("chunks", |b| {
        b.iter(|| {
            for chunk in INPUT.chunks(256) {
                parser.advance(&mut performer, black_box(chunk));
            }
        });
    });

    group.bench_function("sequential", |b| {
        b.iter(|| {
            for &byte in INPUT {
                parser.advance(&mut performer, black_box(&[byte]));
            }
        })
    });

    group.finish()
}

fn parser_advance(c: &mut Criterion) {
    let mut parser = saiga_vte::Parser::new();
    let mut performer = saiga_vte::Performer::default();

    let mut group = c.benchmark_group("saiga parser advance");

    group.bench_function("batch", |b| {
        b.iter(|| {
            parser.advance(&mut performer, black_box(INPUT));
        });
    });

    group.bench_function("chunks", |b| {
        b.iter(|| {
            for chunk in INPUT.chunks(256) {
                parser.advance(&mut performer, black_box(chunk));
            }
        });
    });

    group.bench_function("sequential", |b| {
        b.iter(|| {
            for &byte in INPUT {
                parser.advance(&mut performer, black_box(&[byte]));
            }
        })
    });

    group.finish()
}

criterion_group! {
    name = benches;
    config = Criterion::default().measurement_time(Duration::from_secs(10));
    targets = parser_advance, alacritty_vte
}

criterion_main!(benches);
