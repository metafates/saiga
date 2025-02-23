# Saiga VTE

Parser for implementing virtual terminal emulators in Rust.

The parser is implemented according to [Paul Williams' ANSI parser state
machine]. The state machine doesn't assign meaning to the parsed data and is
thus not itself sufficient for writing a terminal emulator. Instead, it is
expected that an implementation of the `Perform` trait which does something
useful with the parsed data. The `Parser` handles the book keeping, and the
`Perform` gets to simply handle actions.

Derivation of [alacritty/vte] with a focus on being as fast as possible.

## Benchmarks

```raw
saiga parser advance/batch
                        time:   [778.88 µs 779.32 µs 779.72 µs]
saiga parser advance/batch utf8
                        time:   [49.776 µs 50.129 µs 50.479 µs]

alacritty parser advance/batch
                        time:   [841.46 µs 842.35 µs 843.06 µs]
alacritty parser advance/batch utf8
                        time:   [243.94 µs 244.25 µs 244.59 µs]
```

[Paul Williams' ANSI parser state machine]: https://vt100.net/emu/dec_ansi_parser
[alacritty/vte]: https://github.com/alacritty/vte
