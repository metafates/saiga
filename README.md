# Saiga

SIMD+GPU accelerated terminal emulator. Work in progress.

It is heavily based on [Alacritty](https://github.com/alacritty/alacritty) terminal
with some ideas taken from [Ghostty](https://github.com/ghostty-org/ghostty).

It uses SIMD accelerated VTE parser which applies various
optimizations for processing UTF-8 in parallel.

Frontend currently uses [iced](https://iced.rs) based on the implementation by [iced_term](https://github.com/Harzu/iced_term).

However, I plan to use WebGPU directly to implement partial screen rerendering.
