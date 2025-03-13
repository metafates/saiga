pgo_data_dir := "/tmp/pgo-data"
pgo_merged := pgo_data_dir / "merged.profdata"

# Run debug (slow) build
run:
	RUST_LOG=trace cargo run

# Build an optimized binary
build:
    cargo build --release 

# Run in profile guided optimizations generation mode
generate-pgo:
    rm -rf {{ pgo_data_dir }}
    RUSTFLAGS="-Cprofile-generate={{ pgo_data_dir }}" cargo run --release 
    llvm-profdata merge -o {{ pgo_merged }} {{ pgo_data_dir }}

# Build an optimized binary with generated profile guided optimization data
build-pgo:
    RUSTFLAGS="-Cprofile-use={{ pgo_merged }}" cargo build --release 

# Run benchmarks
bench: generate-bench-data && cleanup-bench-data
    cargo bench -p saiga_bench

[private]
generate-bench-data:
    ./saiga_bench/benches/vte/cursor_motion/benchmark >                 ./saiga_bench/benches/vte/cursor_motion/out
    ./saiga_bench/benches/vte/dense_cells/benchmark >                   ./saiga_bench/benches/vte/dense_cells/out
    ./saiga_bench/benches/vte/light_cells/benchmark >                   ./saiga_bench/benches/vte/light_cells/out
    ./saiga_bench/benches/vte/medium_cells/benchmark >                  ./saiga_bench/benches/vte/medium_cells/out
    ./saiga_bench/benches/vte/scrolling/benchmark >                     ./saiga_bench/benches/vte/scrolling/out
    ./saiga_bench/benches/vte/scrolling_bottom_region/benchmark >       ./saiga_bench/benches/vte/scrolling_bottom_region/out
    ./saiga_bench/benches/vte/scrolling_bottom_small_region/benchmark > ./saiga_bench/benches/vte/scrolling_bottom_small_region/out
    ./saiga_bench/benches/vte/scrolling_fullscreen/benchmark >          ./saiga_bench/benches/vte/scrolling_fullscreen/out
    ./saiga_bench/benches/vte/scrolling_top_region/benchmark >          ./saiga_bench/benches/vte/scrolling_top_region/out
    ./saiga_bench/benches/vte/scrolling_top_small_region/benchmark >    ./saiga_bench/benches/vte/scrolling_top_small_region/out
    ./saiga_bench/benches/vte/sync_medium_cells/benchmark >             ./saiga_bench/benches/vte/sync_medium_cells/out
    ./saiga_bench/benches/vte/unicode/benchmark >                       ./saiga_bench/benches/vte/unicode/out
    ./saiga_bench/benches/vte/ascii_all/benchmark >                     ./saiga_bench/benches/vte/ascii_all/out
    ./saiga_bench/benches/vte/ascii_printable/benchmark >               ./saiga_bench/benches/vte/ascii_printable/out
    ./saiga_bench/benches/vte/missing_glyphs/benchmark >                ./saiga_bench/benches/vte/missing_glyphs/out
    ./saiga_bench/benches/vte/no_print/benchmark >                      ./saiga_bench/benches/vte/no_print/out

[private]
cleanup-bench-data:
    rm -f ./saiga_bench/benches/**/out

# Run tests
test:
    cargo test
