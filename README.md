# Pixie Wrangler

Help the Pixies overcome The Resistance¹ in their journey from Source to Sink.

An exciting² blend of traffic simulation games and printed circuit board design.

---

¹Super janky circuit design software

²Debatable

## Play Online

A web build is hosted on [itch.io](https://euclidean-whale.itch.io/pixie-wrangler).

## Build

Pixie wrangler uses the [Bevy](https://bevyengine.org/) engine and is pretty easy to build.

### Dependencies

- [Rust](https://www.rust-lang.org/tools/install)
- Bevy's [dependencies](https://bevyengine.org/learn/quick-start/getting-started/setup/#installing-os-dependencies)

### Native

```bash
cargo run
cargo run --release
cargo run --profile dist
```

### Web

```bash
cargo install --git https://github.com/TheBevyFlock/bevy_cli --tag cli-v0.1.0-alpha.1 --locked bevy_cli
bevy run web
bevy run --release web
bevy build --yes --release --profile web-dist web --bundle
```

## Contributing

Do it! Throw some code at me! Here are some ideas:

## TODO

- [ ] Sound Effects
- [ ] Darken pixies when traveling on lower layers
- [ ] Automatically exit active line drawing mode at intersections
- [ ] Randomizer mode / procgen levels
- [ ] Add something to segment corners to indicate that they will block lines
- [ ] Add "reset all data" button to level select screen
- [ ] Add "export data" button for web users
- [ ] Optimize pixie collision detection
- [ ] More Levels
- [ ] Level editor
- [ ] Pixie-combiners
- [ ] Completely rethink scoring
- [ ] Obstacles that only affect particular layers
- [ ] Nicer color theme
- [ ] VFX, bloom etc
