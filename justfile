run:
	RUST_LOG=trace cargo run

build:
    RUSTFLAGS="-Ctarget-cpu=native" cargo build --release 

run-pgo:
    RUSTFLAGS="-Ctarget-cpu=native -Cprofile-generate=/tmp/pgo-data" cargo run --release 

merge-pgo:
    llvm-profdata merge -o /tmp/pgo-data/merged.profdata /tmp/pgo-data

build-pgo:
    RUSTFLAGS="-Ctarget-cpu=native -Cprofile-use=/tmp/pgo-data/merged.profdata" cargo build --release 
