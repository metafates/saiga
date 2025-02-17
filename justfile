pgo_data_dir := "/tmp/pgo-data"
pgo_merged := pgo_data_dir / "merged.profdata"

run:
	RUST_LOG=trace cargo run

build:
    cargo build --release 

generate-pgo:
    rm -rf {{ pgo_data_dir }}
    RUSTFLAGS="-Cprofile-generate={{ pgo_data_dir }}" cargo run --release 

merge-pgo:
    llvm-profdata merge -o {{ pgo_merged }} {{ pgo_data_dir }}

build-pgo:
    RUSTFLAGS="-Cprofile-use={{ pgo_merged }}" cargo build --release 

bench:
    cargo bench

[macos]
bench-results:
    open ./target/criterion/report/index.html

[linux]
bench-results:
    xdg-open ./target/criterion/report/index.html

test:
    cargo test
