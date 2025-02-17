use std::time::Duration;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use saiga_vte::{param::Params, Executor, Parser};

#[derive(Default)]
struct NopExecutor {}

impl Executor for NopExecutor {
    fn print(&mut self, _c: char) {}

    fn execute(&mut self, _byte: u8) {}

    fn put(&mut self, _byte: u8) {}

    fn hook(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, _action: char) {}

    fn unhook(&mut self) {}

    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {}

    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {}

    fn csi_dispatch(
        &mut self,
        _params: &Params,
        _intermediates: &[u8],
        _ignore: bool,
        _action: char,
    ) {
    }
}

const INPUT: &[u8] = include_bytes!("bench.ansi");

fn parser_advance(c: &mut Criterion) {
    let mut parser = Parser::new();
    let mut executor = NopExecutor::default();

    let mut group = c.benchmark_group("parser advance");

    group.bench_function("batch", |b| {
        b.iter(|| {
            parser.advance(&mut executor, black_box(INPUT));
        });
    });

    group.bench_function("chunks", |b| {
        b.iter(|| {
            for chunk in INPUT.chunks(256) {
                parser.advance(&mut executor, black_box(chunk));
            }
        });
    });

    group.bench_function("sequential", |b| {
        b.iter(|| {
            for &byte in INPUT {
                parser.advance(&mut executor, black_box(&[byte]));
            }
        })
    });

    group.finish()
}

criterion_group! {
    name = benches;
    config = Criterion::default().measurement_time(Duration::from_secs(10));
    targets = parser_advance
}

criterion_main!(benches);
