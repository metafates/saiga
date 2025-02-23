# Saiga

<img width="1212" alt="Screenshot 2025-02-01 at 19 23 32" src="https://github.com/user-attachments/assets/9c057ef1-251a-4244-bd4a-4ec4d90dea51" />

SIMD+GPU accelerated terminal emulator. Work in progress.

It is heavily based on [Alacritty](https://github.com/alacritty/alacritty) terminal
with some ideas taken from [Ghostty](https://github.com/ghostty-org/ghostty).

- SIMD accelerated VTE parser which applies various
optimizations for processing UTF-8 in parallel.
- WebGPU frontend with damage tracking and partial screen updates

## Performance

Saiga aims to be fast.

Right now it lacks many useful (and not so) features and optimizations.
However, it can already outperform Alacritty terminal on [alacritty/vtebench](https://github.com/alacritty/vtebench) under certain circumstances.

<img width="700" alt="vtebench results" src="https://github.com/user-attachments/assets/a8760b7b-ffcf-4b11-acce-cc9e8fbe0394">

The screenshot above demonstrates results of the benchmark.
Alacritty is on the left, Saiga is on the right.
Apple M3 Pro, 36 GB RAM compiled with PGO.

The results of the benchmark may vary, as the Saiga is work in progress project.

## Building

You will need rust stable toolchain.

Install [just](https://github.com/casey/just) command runner

Build without profile guided optimizations.
_You might want them, as they give significant performance boost_

```bash
just build

# You can then run saiga like that
./target/release/saiga
```

Build with profile guided optimizations:

> [!IMPORTANT]
> On macOS you will need to install latest `llvm` tools: `brew install llvm` **AND** follow the instructions it will give you after the installation, like properly adding it to the `$PATH`

```bash
# First, you need to generate a profile. To do so, run
just generate-pgo

# It will compile and run saiga in special mode for generating PGO data.
# Do something with it you would normally do with terminal, like using vim.
# You can also run vtebench with it.
# After you're done recording your profile close the terminal.

# And compile with it
just build-pgo

# You can then run saiga like that
./target/release/saiga
```

## TODO

- Proper input handling. Right now it's very basic and does not handle some key sequences. Mouse support is planned too (including text selection).
- Basic configuration
- Fix renderer issues, like rects overflowing each other
- Apply more optimizations
