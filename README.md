# Saiga

<img width="1212" alt="Screenshot 2025-02-01 at 19 23 32" src="https://github.com/user-attachments/assets/9c057ef1-251a-4244-bd4a-4ec4d90dea51" />

SIMD+GPU accelerated terminal emulator. Work in progress.

It is heavily based on [Alacritty](https://github.com/alacritty/alacritty) terminal
with some ideas taken from [Ghostty](https://github.com/ghostty-org/ghostty).

- SIMD accelerated VTE parser which applies various
optimizations for processing UTF-8 in parallel.
- WebGPU frontend with damage tracking and partial screen updates

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

## Benchmarks

```bash
just bench
```

## Tests

```bash
just test
```

## TODO

- Proper input handling. Right now it's very basic and does not handle some key sequences. Mouse support is planned too (including text selection).
- Basic configuration
- Fix renderer issues, like rects overflowing each other
- Apply more optimizations
