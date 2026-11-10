# Nylium for SteelMC

This crate embeds [SteelMC](https://github.com/Steel-Foundation/SteelMC) directly in Nylium. It includes a library adapter and the headless `nylium-steel` executable. It has its own Cargo workspace and lockfile; its default build does not compile Nylium, Ely or GPUI.

```sh
# From a server directory; configuration and saves belong to this directory.
nix run /path/to/Nylium#steel

# From the Nylium checkout, for development:
nix develop -c cargo run --manifest-path nylium-steel/Cargo.toml -- config/config.toml

# Optional graphical launcher:
nix develop -c cargo run --manifest-path nylium-steel/Cargo.toml --features ui --bin nylium-steel-ui -- config/config.toml
```

The headless launcher starts the server, prints logs and saves/stops on Ctrl-C.

The optional first argument is the Steel configuration path. Steel's relative paths, including `worlds.toml`'s `save_path`, remain relative to the process working directory. Choose the working directory before launching; selecting a configuration file does not change it.

## Embedding

```rust,ignore
use nylium::Nylium;
use nylium_steel::SteelAdapter;
use tracing_subscriber::prelude::*;

let adapter = SteelAdapter::new("config/config.toml")?;
tracing_subscriber::registry()
    .with(adapter.logging_layer())
    .with(tracing_subscriber::EnvFilter::new("info"))
    .try_init()?;
adapter.install_panic_handler();
Nylium::run(adapter);
```

Enable the optional `ui` feature for the embedding example above. The library supplies a tracing layer instead of replacing the host's subscriber. The optional panic handler chains the host hook and cancels Steel when a worker panics; the executable installs it, as Steel's own launcher does. Enable tracing-subscriber's `tracing-log` support (enabled by default) to bridge Steel's `log` events. The executable installs the layer and Steel's text-component resolver for console feedback. It does not start Steel's interactive terminal logger; logs appear in Nylium.

## Behavior

- Start creates Steel's main and chunk runtimes on a dedicated thread. Restart reloads configuration and recreates the runtimes, so thread-count changes apply too. Stop cancels the server, drains tracked tasks, and invokes Steel's save-and-shutdown path before acknowledging completion.
- Commands use Steel's console queue. Success acknowledges queue admission; command parsing and execution feedback appears in the console. There is no upstream result-returning command API.
- Player state combines Steel's profile cache with connected players in each loaded world. OP state comes from Steel's persisted `op` permission group.
- Every loaded world's native game rules are exposed with their actual types and bounds. Changes are applied to that world and its level data is saved before acknowledgement.
- Existing scalar fields under `[server]` in `config.toml` are editable. Writes preserve TOML comments and unrelated tables, validate a candidate with Steel's own loader, and atomically replace the configuration. Changes apply after restart. Signed integers use text fields because Nylium's numeric fields hold unsigned values.
- Arrays, server links, optional fields not already present, logging configuration, groups, and world definitions remain editable in their TOML files. The adapter does not synthesize unsupported controls for those values.
- Steel exposes no general change subscription, so the adapter samples players and game rules every 250 ms, observes external configuration edits, and samples CPU/RAM once per second. Logs are pushed as events. Histories retain 1,000 logs and 120 metric samples.

## Building

Use `nix develop` or the exact nightly in `rust-toolchain.toml`. Steel uses nightly language features. The flake fetches the Minecraft build resources using the official URL and SHA256 pinned in `nix/minecraft-assets.json` and prepares writable build-local copies for its code generators. The `steel-source` flake input and the Cargo Steel dependencies must refer to the same revision. The local asset pin supplies Minecraft 26.3 because the upstream Nix metadata currently points to a 26.2 JAR. The build verifies the JAR version before using it.

The Nix `steel` package is tested on Linux. `cargo test --manifest-path nylium-steel/Cargo.toml` includes a real flat-world server lifecycle test in a temporary directory with an ephemeral port; it does not touch an existing server.
