# Pixie Wrangler

Help the Pixies overcome The Resistance¹ in their journey from Source to Sink.

An exciting² blend of traffic simulation games and printed circuit board design.

It's entirely possible that there's a playable demo at [itch.io](https://euclidean-whale.itch.io/pixie-wrangler) or [pixiewrangler.robparrett.com](https://pixiewrangler.robparrett.com).

---

¹Super janky circuit design software

²Debatable

## TODO

- [ ] Audio
- [ ] Darken pixies when traveling on lower layers
- [ ] Automatically stop line drawing at intersections
- [ ] Randomizer mode?
- [ ] Add something to segment corners to indicate that they will block lines
- [ ] Add "reset all data" button to level select screen
- [ ] Add "export data" button for web users
- [ ] Optimize pixie collision detection
- [ ] More Levels
- [ ] Pixie-combiners
- [ ] Completely rethink scoring
- [ ] Obstacles that only affect particular layers

## Contributing

Do it! Throw some code at me!

## Prerequisites

```bash
cargo install cargo-make
```

## Build and serve WASM version

```bash
cargo make serve
cargo make --profile release serve
```

then point your browser to [http://127.0.0.1:4001/](http://127.0.0.1:4001/)

## Build and run native version

```bash
cargo make run
cargo make --profile release run
```
