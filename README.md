# Pixie Wrangler

Help the Pixies overcome The Resistance¹ in their journey from Source to Sink.

An exciting² blend of traffic simulation games and printed circuit board design.

---

¹Super janky circuit design software

²Debateable

## TODO

- [ ] Audio
- [ ] Darken pixies when traveling on lower layers
- [ ] Automatically stop line drawing at intersections
- [ ] Randomizer mode?
- [ ] Add something to corners to indicate that they will block lines

## Prerequisites

```bash
cargo install cargo-make
```

## Build and serve WASM version

```bash
cargo make serve
cargo make serve --profile release
```

then point your browser to [http://127.0.0.1:4001/](http://127.0.0.1:4001/)

## Build and run native version

```bash
cargo make run
cargo make run --profile release
```
