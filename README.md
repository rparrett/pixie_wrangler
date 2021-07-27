# Pixie Wrangler

Help the pixies overcome The Resistance in their journey from Source to Sink.

An exciting blend of traffic simulation games and printed circuit board design.

## TODO

- [ ] Traffic congestion between differently flavored pixies
- [ ] I would like to know my previous best score for a particular level
- [ ] If (grayed) release button is clicked, we should highlight the offending
      OUT node
- [ ] Pixies should move to lower z values when traveling on lower layers
- [ ] It would be nice if pixies could darken when traveling on lower layers
- [ ] Maybe line drawing should automatically stop when drawing to an intersection
- [ ] Randomize obstacles?

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
