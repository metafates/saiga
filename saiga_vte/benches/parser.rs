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

impl vtparse::VTActor for NopPerformer {
    fn print(&mut self, b: char) {}

    fn execute_c0_or_c1(&mut self, control: u8) {}

    fn dcs_hook(
        &mut self,
        mode: u8,
        params: &[i64],
        intermediates: &[u8],
        ignored_excess_intermediates: bool,
    ) {
    }

    fn dcs_put(&mut self, byte: u8) {}

    fn dcs_unhook(&mut self) {}

    fn esc_dispatch(
        &mut self,
        params: &[i64],
        intermediates: &[u8],
        ignored_excess_intermediates: bool,
        byte: u8,
    ) {
    }

    fn csi_dispatch(&mut self, params: &[vtparse::CsiParam], parameters_truncated: bool, byte: u8) {
    }

    fn osc_dispatch(&mut self, params: &[&[u8]]) {}

    fn apc_dispatch(&mut self, data: Vec<u8>) {}
}

const BAT_OUTPUT: &[u8] = include_bytes!("bat.txt");
const BIG_UTF8: &[u8] = include_bytes!("utf8.txt");
const BIG_ASCII: &[u8] = include_bytes!("ascii.txt");

fn wezterm_vte(c: &mut Criterion) {
    let mut parser = vtparse::VTParser::new();
    let mut actor = NopPerformer::default();

    let mut group = c.benchmark_group("wezterm parser advance");

    group.bench_function("batch", |b| {
        b.iter(|| {
            parser.parse(black_box(BAT_OUTPUT), &mut actor);
        });
    });

    group.bench_function("batch utf8", |b| {
        b.iter(|| {
            parser.parse(black_box(BIG_UTF8), &mut actor);
        });
    });

    group.bench_function("batch ascii", |b| {
        b.iter(|| {
            parser.parse(black_box(BIG_ASCII), &mut actor);
        });
    });

    group.finish()
}

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

    group.bench_function("batch ascii", |b| {
        b.iter(|| {
            parser.advance(&mut performer, black_box(BIG_ASCII));
        });
    });

    group.finish()
}

fn saiga_vte(c: &mut Criterion) {
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

    group.bench_function("batch ascii", |b| {
        b.iter(|| {
            parser.advance(&mut performer, black_box(BIG_ASCII));
        });
    });

    group.finish()
}

criterion_group! {
    name = benches;
    config = Criterion::default().measurement_time(Duration::from_secs(10)).with_profiler(PProfProfiler::new(50_000, Output::Flamegraph(None)));
    targets = saiga_vte, alacritty_vte, wezterm_vte,
}

criterion_main!(benches);
