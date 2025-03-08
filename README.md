# Chust

**Chust** is a Rust-based tool that leverages machine learning to detect chessboards and pieces from images. It can extract Forsyth-Edwards Notation (FEN) from a chessboard image, mark detected pieces, and even play a game of chess for you as a bot.

## Features

- **Bot Gameplay** - A chess bot; plays chess automatically for you.
- **Chessboard and Piece Detection** - Extracts the position of chess pieces from an image.
- **FEN Generation** - Converts detected chessboard positions into FEN notation.
- **Visual Marking** - Highlights detected pieces for verification.
- **Refined Search Mode** - Enhances accuracy by performing a secondary detection on a cropped chessboard.

## Installation

### Prebuilt Binaries
You can find prebuilt binaries for Windows and Linux on the [Releases](https://github.com/Zai-Kun/chust/releases) page. For macOS and other platforms, you will need to compile Chust manually.

### Compiling from Source
Ensure you have Rust and Cargo installed. On Windows, you also need [Visual Studio 2022 (≥ 17.11)](https://visualstudio.microsoft.com/) installed.

```sh
git clone https://github.com/Zai-Kun/chust && cd chust
cargo build --release
```

The compiled binary will be available in the `./target/release` folder.

#### Optional: Embedding the Model
To embed the machine learning model directly into Chust, use:

```sh
cargo build --release --features embed_model
```

Make sure the model is downloaded and placed in the Chust directory before building.

## Usage

Chust provides two primary commands:
- `process` - Analyze an image for chessboard detection and FEN extraction.
- `play` - Play a game of chess automatically.

### Requirements

#### Machine Learning Model
*Not needed if compiled with `embed_model`. If using a prebuilt binary, this is required.*

Download the ONNX model from the [2D Chess Pieces Detection](https://github.com/Zai-Kun/2d-chess-pieces-detection/releases) page. Ensure it is saved as `chess_detection.onnx` in the same directory as Chust or specify its path with `--model-path`.

#### Stockfish Engine (Optional, for `play` command)
Stockfish is required for Chust to play chess as a bot.

- Download from [Stockfish Chess](https://stockfishchess.org/download/).
- On Arch Linux, install via `yay -S stockfish`.

### Commands & Examples

#### Play a Game as a Bot

```sh
chust play
```

##### Options:
- `--castle-w` - Enable castling for white.
- `--castle-b` - Enable castling for black.
- `--pov` - Choose to play as black (`b`) or white (`w`).
- `--stockfish-path` - Path to the Stockfish engine executable.
- `--stockfish-depth` - Depth for Stockfish analysis.
- `--model-path` - Path to the machine learning model.

##### Platform-Specific Customization:
If Chust does not support automatic screen capturing and clicking on your OS, you can specify custom commands:

- `--screenshot-command` - Command that outputs a screenshot to stdout for Chust to process.
- `--click-command` - Command to simulate a mouse click at coordinates `{x}` and `{y}`.

For more details:
```sh
chust play --help
```

#### Example: Play Blitz
```sh
chust play --pov w --screenshot-delay=0.3 --stockfish-depth=10
```

#### Example: Normal Game with Animation Lag
```sh
chust play --pov w --screenshot-delay=0.4 --stockfish-depth=20 --recheck-after-change
```

#### Example: Enable Castling
```sh
chust play --castle-w --castle-b
```

# Known Issues

* **Promotion is not automatic**: If a pawn reaches the last rank, you will need to manually promote it.
* **False move detection**: Sometimes Chust may think that the opponent has moved when they actually haven't. If this happpens to you, you may need to adjust `--screenshot-delay` and you may need to add `--recheck-after-change`.
* **Model failure**: The model isn't perfect and it may fail in some cases but that rarely happens.

## License
This project is licensed under the MIT License.

## Contributing
Contributions are welcome! Feel free to open issues and submit pull requests.

---

*Made with ❤️ in Rust by Zai.*
