# nes-rs

A NES emulator with native Mac/Windows frontends and a static WebAssembly frontend.

## Browser build

Install Rust and its Wasm target, then build:

```sh
rustup target add wasm32-unknown-unknown
./scripts/build-web.sh
python3 -m http.server 8080 --directory dist
```

Visit http://localhost:8080. Serve the files over HTTP rather than opening
index.html directly. The build needs no Node packages or JavaScript binding tools.
All paths are relative, so the site also works under a GitHub Pages repository path.

Choose a local `.nes` ROM to start. ROM bytes stay in the browser and are never
uploaded. The current emulator supports iNES mapper 2 (UxROM) cartridges with
CHR RAM and at least two PRG banks; other formats show an error.

Controls match the desktop app: arrows to move, Z for A, X for B,
Right Shift for Select, and Enter for Start. The page has pause and mute controls,
clears held keys on focus loss, and stops emulating while the tab is hidden.
Sound starts after selecting a ROM when permitted by the browser; video also
works without audio. Choosing another file resets the emulator.

## GitHub Pages

In the GitHub repository, open **Settings → Pages → Build and deployment** and
select **GitHub Actions** as the source. The `Release` workflow builds Mac,
Windows, and browser artifacts when a version tag such as `v0.0.2` is pushed.
The tagged commit must already be on the default branch (`master`).
After all builds succeed, the workflow publishes the GitHub release with all
three downloads, then deploys that same browser build to Pages. Regular branch
pushes and pull requests do not run the release workflow or deploy the site.
The deployment job reports the hosted URL in the `github-pages` environment.

The native release workflow and its packaging artwork are restored from
`v0.0.1`, where they existed before being removed from the current default branch.

The uploaded Pages artifact contains HTML, JavaScript, and the Wasm binary;
no ROMs are bundled. Deployment needs the repository's Pages feature enabled
and any environment protection rules satisfied.

## Native build

```sh
cargo run --release -- path/to/game.nes
cargo test --lib
```

Native builds retain minifb for the window and bundled SDL2 for audio, and need
the corresponding platform build tools (including CMake for SDL2). These
dependencies are excluded from the Wasm target.
