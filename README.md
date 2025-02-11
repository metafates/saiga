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

Right now it lacks many useful (and not so) features and optimizations. However, it can already outperform Alacritty terminal on [alacritty/vtebench](https://github.com/alacritty/vtebench)

<img width="700" alt="vtebench results" src="https://github.com/user-attachments/assets/a8760b7b-ffcf-4b11-acce-cc9e8fbe0394">

The screenshot above demonstrates results of the benchmark. 
Alacritty is on the left, Saiga is on the right. Apple M3 Pro, 36 GB RAM compiled with PGO
