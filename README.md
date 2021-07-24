# Pixie Wrangler

Help the pixies overcome The Resistance in their journey from Source to Sink.

An exciting blend of traffic simulation games and printed circuit board design.

## TODO

- [ ] Pressing escape should cancel line drawing
- [ ] Drawing to a Terminus should cancel drawing
- [ ] Maybe the cursor should turn white when not drawing?
- [ ] Traffic congestion between differently flavored pixies
- [ ] Acute and right angles should slow down pixies

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
