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
uploaded. Desktop and browser builds share support for 28 mapper IDs, including
NROM, UxROM, CNROM, AxROM, MMC1, MMC2/MMC4, MMC3/MMC6, and common discrete
boards. See [supported cartridges](docs/mappers.md) for IDs, memory layouts,
submapper variants, and validation.

Legacy iNES and NES 2.0 headers are accepted. NES 2.0 timing must be
NTSC-compatible; extended ROM sizes and unimplemented board layouts are
rejected. CHR-ROM is read-only, CHR RAM is writable, and supported boards
provide banked PRG RAM with their enable/protection controls. Save RAM currently
lasts only for the running cartridge: reloading or choosing another ROM clears it.

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

## Continuous integration

CI runs on pull requests, pushes to `master`, and manual dispatch. It builds
and tests the native targets on Linux and independently builds the browser site.
A Node smoke test loads the compiled Wasm module and checks cartridge validation,
video, audio, controller input, and replacing a running cartridge using a synthetic
ROMs across all supported mapper IDs; no external ROM is required. CI never publishes a release or deploys Pages.

Legacy formatting and compiler warnings are not release blockers in this workflow.
The current codebase predates modern Rust formatting and lint conventions; those
can be addressed separately rather than preventing builds and tests from running.

## Bumping a version and releasing

Open **Actions → Bump version → Run workflow**, select the default branch
(`master`), then choose a bump and release channel. The workflow updates the
root package versions in both `Cargo.toml` and `Cargo.lock`, commits
the change, and atomically pushes the commit and matching `vX.Y.Z` tag. It then
explicitly dispatches the existing Release workflow for that tag. This explicit
dispatch is needed because pushes made with `GITHUB_TOKEN` do not trigger
other workflows. No additional personal access token is required.

Run the bump after the code has passed normal CI. The bump workflow performs
version and tag consistency checks, then commits, tags, and starts release builds.
It does not rerun CI or the version-helper tests.

| Bump | Channel | Example |
| --- | --- | --- |
| patch | stable | `0.1.0 → 0.1.1` |
| minor | stable | `0.1.2 → 0.2.0` |
| major | stable | `0.1.2 → 1.0.0` |
| minor | alpha | `0.1.0 → 0.2.0-alpha.1` |
| prerelease | alpha | `0.2.0-alpha.1 → 0.2.0-alpha.2` |
| prerelease | beta | `0.2.0-alpha.2 → 0.2.0-beta.1` |
| prerelease | rc | `0.2.0-beta.1 → 0.2.0-rc.1` |
| promote | stable | `0.2.0-rc.1 → 0.2.0` |

Patch/minor/major always increment that numeric component and start the chosen
prerelease channel at `.1` if selected. Use `prerelease` to continue the current
prerelease or move forward from alpha to beta to rc; from a stable version it
starts a prerelease of the next patch. Use `promote` with `stable` to remove the
suffix without incrementing the numeric version.

Prereleases publish Mac, Windows, and browser downloads marked as GitHub
prereleases. They neither become the latest stable release nor deploy to Pages.
Stable releases update Pages after publishing. The Release workflow validates
that the tag exactly matches the manifest version, including the prerelease suffix.

Existing tags and mismatched manifest/lockfile versions stop the bump. If the
default branch moves before the push, the workflow stops rather
than tagging a different revision; rerun it against the updated branch.
Repository rules must allow the workflow token to push the version commit and
tag. If the push succeeds but release dispatch fails, manually run **Release**
against the new tag (for example `gh workflow run release.yml --ref v0.1.1`);
do not bump again just to retry a build.
