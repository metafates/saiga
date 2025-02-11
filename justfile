pgo_data_dir := "/tmp/pgo-data"
pgo_merged := pgo_data_dir / "merged.profdata"

run:
	RUST_LOG=trace cargo run

build:
    cargo build --release 

generate-pgo:
    rm -rf /tmp/pgo-data
    RUSTFLAGS="-Cprofile-generate={{ pgo_data_dir }}" cargo run --release 

merge-pgo:
    llvm-profdata merge -o /tmp/pgo-data/merged.profdata /tmp/pgo-data

build-pgo:
    RUSTFLAGS="-Cprofile-use={{ pgo_merged }}" cargo build --release 
