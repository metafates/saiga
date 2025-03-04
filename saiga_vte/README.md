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

Saiga

```raw
saiga parser advance/batch
                        time:   [778.88 µs 779.32 µs 779.72 µs]
saiga parser advance/batch utf8
                        time:   [45.569 µs 45.821 µs 46.070 µs]
saiga parser advance/batch ascii
                        time:   [11.568 µs 11.579 µs 11.592 µs]
```

Alacritty

```raw
alacritty parser advance/batch
                        time:   [868.27 µs 868.46 µs 868.68 µs]
alacritty parser advance/batch utf8
                        time:   [245.58 µs 245.84 µs 246.12 µs]
alacritty parser advance/batch ascii
                        time:   [517.98 µs 518.19 µs 518.40 µs]
```

[Paul Williams' ANSI parser state machine]: https://vt100.net/emu/dec_ansi_parser
[alacritty/vte]: https://github.com/alacritty/vte
