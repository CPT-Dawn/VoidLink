# Contributing

Thanks for helping. Bug reports, fixes and ideas are all welcome; for a larger change, open an issue first so we can agree on the shape.

## Setup

You need Rust 1.88 or newer, `pkg-config` and the D-Bus headers (`libdbus-1-dev`, `dbus-devel` or `dbus`).

```sh
git clone https://github.com/CPT-Dawn/VoidLink
cd VoidLink
cargo build
```

## Run without Bluetooth hardware

The `demo` feature swaps BlueZ for a scripted fake adapter with invented devices:

```sh
VOIDLINK_DEMO=1 cargo run --features demo
```

Against real hardware, `cargo run` needs BlueZ with `bluetooth.service` running.

## Checks

CI runs these on every push and pull request; run them before you open one:

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo clippy --locked --all-targets --features demo -- -D warnings
cargo test --locked --features demo
```

CI also builds with Rust 1.88 (`cargo check --locked`) to hold the minimum supported version.

## Re-recording the demo

The README demo comes from [`demo/voidlink.tape`](demo/voidlink.tape), recorded with [VHS](https://github.com/charmbracelet/vhs) and the demo adapter, so it shows no real devices or paths.

```sh
cargo build --release --features demo --target-dir target/demo
vhs demo/voidlink.tape
ffmpeg -i assets/demo.mp4 -vf "fps=25,scale='min(2560,iw)':-1:flags=lanczos,split[a][b];[a]palettegen=max_colors=256:stats_mode=diff[p];[b][p]paletteuse=dither=none:diff_mode=rectangle" -loop 0 assets/demo.gif
```

## Pull requests

1. Branch from `main`.
2. Use [conventional commits](https://www.conventionalcommits.org/) (`fix(ui): …`, `feat: …`).
3. For anything a user would notice, add an entry under `## [Unreleased]` at the top of [CHANGELOG.md](CHANGELOG.md), creating the heading if it isn't there.
4. Open the PR and describe what changed and how you checked it.

[Architecture](docs/architecture.md) explains how the code is laid out.
