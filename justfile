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
bench:
    {{ just_executable() }} --justfile {{ "saiga_vte" / "justfile" }} generate-bench-data
    cargo bench

# Run tests
test:
    cargo test
