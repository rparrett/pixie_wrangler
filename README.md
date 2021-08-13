# Pixie Wrangler

Help the pixies overcome The Resistance in their journey from Source to Sink.

An exciting blend of traffic simulation games and printed circuit board design.

## TODO

- [ ] At least two more levels
- [ ] Corner debuff should be distance rather than time based
- [ ] We should save the player's solution to each level
- [ ] Audio
- [ ] It would be nice if pixies could darken when traveling on lower layers
- [ ] Maybe line drawing should automatically stop when drawing to an intersection
      Should it just always stop unless there are no collisions at the endpoint
      At all?
- [ ] Randomizer mode?

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
