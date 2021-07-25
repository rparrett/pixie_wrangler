# Pixie Wrangler

Help the pixies overcome The Resistance in their journey from Source to Sink.

An exciting blend of traffic simulation games and printed circuit board design.

## TODO

- [ ] Traffic congestion between differently flavored pixies
- [ ] Add "rip up net" tool (no undo)
- [ ] Add buttons for first/last layer

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
