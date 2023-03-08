# Pixie Wrangler

Help the Pixies overcome The Resistance¹ in their journey from Source to Sink.

An exciting² blend of traffic simulation games and printed circuit board design.

---

¹Super janky circuit design software

²Debatable

## Play Online

A web build is hosted on [itch.io](https://euclidean-whale.itch.io/pixie-wrangler).

## Build

Pixie wrangler uses the [Bevy 0.10](https://bevyengine.org/) engine and is pretty easy to build.

### Dependencies

- [Rust](https://www.rust-lang.org/tools/install)

Bevy also has a few [dependencies](https://bevyengine.org/learn/book/getting-started/setup/) on Windows and Linux that you may need.

### Native

```bash
cargo run --release
```

### Web

```bash
cargo install cargo-make
cargo make --profile release serve
```

## Contributing

Do it! Throw some code at me! Here are some ideas:

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
