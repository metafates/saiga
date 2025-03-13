use std::time::Duration;

use criterion::{Criterion, Throughput, black_box, criterion_group};

#[derive(Default)]
struct NopPerformer {}

impl saiga_vte::Perform for NopPerformer {
    fn print(&mut self, c: char) {
        black_box(c);
    }

    fn execute(&mut self, byte: u8) {
        black_box(byte);
    }

    fn put(&mut self, byte: u8) {
        black_box(byte);
    }

    fn hook(
        &mut self,
        params: &saiga_vte::params::Params,
        intermediates: &[u8],
        ignore: bool,
        action: char,
    ) {
        black_box((params, intermediates, ignore, action));
    }

    fn unhook(&mut self) {
        black_box(());
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], bell_terminated: bool) {
        black_box((params, bell_terminated));
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], ignore: bool, byte: u8) {
        black_box((intermediates, ignore, byte));
    }

    fn csi_dispatch(
        &mut self,
        params: &saiga_vte::params::Params,
        intermediates: &[u8],
        ignore: bool,
        action: char,
    ) {
        black_box((params, intermediates, ignore, action));
    }
}

impl vte::Perform for NopPerformer {
    fn print(&mut self, c: char) {
        black_box(c);
    }

    fn execute(&mut self, byte: u8) {
        black_box(byte);
    }

    fn put(&mut self, byte: u8) {
        black_box(byte);
    }

    fn hook(&mut self, params: &vte::Params, intermediates: &[u8], ignore: bool, action: char) {
        black_box((params, intermediates, ignore, action));
    }

    fn unhook(&mut self) {
        black_box(());
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], bell_terminated: bool) {
        black_box((params, bell_terminated));
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], ignore: bool, byte: u8) {
        black_box((intermediates, ignore, byte));
    }

    fn csi_dispatch(
        &mut self,
        params: &vte::Params,
        intermediates: &[u8],
        ignore: bool,
        action: char,
    ) {
        black_box((params, intermediates, ignore, action));
    }
}

impl vtparse::VTActor for NopPerformer {
    fn print(&mut self, b: char) {
        black_box(b);
    }

    fn execute_c0_or_c1(&mut self, control: u8) {
        black_box(control);
    }

    fn dcs_hook(
        &mut self,
        mode: u8,
        params: &[i64],
        intermediates: &[u8],
        ignored_excess_intermediates: bool,
    ) {
        black_box((mode, params, intermediates, ignored_excess_intermediates));
    }

    fn dcs_put(&mut self, byte: u8) {
        black_box(byte);
    }

    fn dcs_unhook(&mut self) {
        black_box(());
    }

    fn esc_dispatch(
        &mut self,
        params: &[i64],
        intermediates: &[u8],
        ignored_excess_intermediates: bool,
        byte: u8,
    ) {
        black_box((params, intermediates, ignored_excess_intermediates, byte));
    }

    fn csi_dispatch(&mut self, params: &[vtparse::CsiParam], parameters_truncated: bool, byte: u8) {
        black_box((params, parameters_truncated, byte));
    }

    fn osc_dispatch(&mut self, params: &[&[u8]]) {
        black_box(params);
    }

    fn apc_dispatch(&mut self, data: Vec<u8>) {
        black_box(data);
    }
}

fn vte(c: &mut Criterion) {
    let mut performer = NopPerformer::default();

    let mut wezterm_parser = vtparse::VTParser::new();
    let mut alacritty_parser = vte::Parser::new();
    let mut saiga_parser = saiga_vte::Parser::new();

    macro_rules! suite {
        ($name:literal) => {
            ($name, include_bytes!(concat!($name, "/out")) as &[u8])
        };
    }

    for (name, input) in [
        suite!("unicode"),
        suite!("ascii_all"),
        suite!("ascii_printable"),
        suite!("missing_glyphs"),
        suite!("no_print"),
        suite!("cursor_motion"),
        suite!("dense_cells"),
        suite!("light_cells"),
        suite!("medium_cells"),
        suite!("scrolling"),
        suite!("scrolling_bottom_region"),
        suite!("scrolling_bottom_small_region"),
        suite!("scrolling_fullscreen"),
        suite!("scrolling_top_region"),
        suite!("scrolling_top_small_region"),
        suite!("sync_medium_cells"),
    ] {
        let mut group = c.benchmark_group(name);

        // ignore changes smaller than 2.5%
        group.noise_threshold(0.025);

        group.throughput(Throughput::BytesDecimal(input.len() as u64));

        group.bench_with_input("saiga", input, |b, i| {
            b.iter(|| {
                saiga_parser.advance(&mut performer, black_box(i));
            });
        });

        group.bench_with_input("alacritty", input, |b, i| {
            b.iter(|| {
                alacritty_parser.advance(&mut performer, black_box(i));
            });
        });

        group.bench_with_input("wezterm", input, |b, i| {
            b.iter(|| {
                wezterm_parser.parse(black_box(i), &mut performer);
            });
        });

        group.finish();
    }
}

criterion_group! {
    name = vte_bench;
    config = Criterion::default().measurement_time(Duration::from_secs(10));
    targets = vte,
}
