# 🐦 Floopy Birb

A Flappy Bird clone built with [Bevy](https://bevyengine.org/) game engine in Rust.

![Rust](https://img.shields.io/badge/Rust-000000?style=flat&logo=rust&logoColor=white)
![Bevy](https://img.shields.io/badge/Bevy-232326?style=flat&logo=bevy&logoColor=white)

## Features

- 🎮 Classic Flappy Bird gameplay
- 🐤 Animated bird sprite with dynamic rotation (tilts up when flapping, dives down when falling)
- 🏙️ 8 unique city backgrounds with multi-layer parallax scrolling
- 🎲 Random city selection each game
- 🎵 Background music with mute toggle
- 📊 Score tracking
- 🔄 Menu and game over screens

## Controls

| Key | Action |
|-----|--------|
| `Space` | Flap / Start game / Restart |
| `M` | Toggle music on/off |
| `R` | Restart (on game over) |

## Requirements

- [Rust](https://www.rust-lang.org/tools/install) (latest stable)
- Linux: Additional dependencies for Bevy (see [Bevy setup](https://bevyengine.org/learn/quick-start/getting-started/setup/))

## Building & Running

```bash
# Clone the repository
git clone https://github.com/yourusername/floopybirb.git
cd floopybirb

# Run the game
cargo run --release
```

> **Note:** The first build may take a few minutes as it compiles Bevy and its dependencies.

## Project Structure

```
floopybirb/
├── src/
│   └── main.rs          # Game logic
├── assets/
│   ├── textures/
│   │   ├── bird.png     # Bird sprite sheet
│   │   └── city 1-8/    # City backgrounds (5-6 parallax layers each)
│   │       ├── 1.png    # Furthest layer (sky)
│   │       ├── 2.png
│   │       └── ...      # Closer layers
│   └── music/
│       └── music.ogg    # Background music
├── Cargo.toml
└── README.md
```

## Dependencies

- [Bevy](https://crates.io/crates/bevy) - Game engine
- [rand](https://crates.io/crates/rand) - Random number generation for pipe placement

## License

This project is open source and available under the [MIT License](LICENSE).

## Acknowledgments

- Inspired by the original Flappy Bird by Dong Nguyen
- Built with the amazing Bevy game engine