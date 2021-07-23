# Pixie Wrangler

Help the pixies overcome The Resistance in their journey from Source to Sink.

An exciting blend of traffic simulation games and printed circuit board design.

## TODO

- [ ] Draw a `/` on layer 1. Cross with with a `\` on layer 2. If a line is drawn
      from the intersection, both lines get split and connected to the new line.
      This should not be possible.

## Prerequisites

```
cargo install cargo-make
```

## Build and serve WASM version

```
cargo make serve
```

then point your browser to http://127.0.0.1:4000/

## Build and run native version

```
cargo make run
```
