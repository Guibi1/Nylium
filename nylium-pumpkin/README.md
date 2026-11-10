# Nylium adapter for PumpkinMC

This crate embeds PumpkinMC and publishes its server state through `nylium-adapter`.
It has its own Cargo workspace and lockfile. Its default build contains no Nylium GUI, Ely, or GPUI.

```sh
# From a server directory:
nix run /path/to/Nylium#pumpkin-server
# Optional graphical launcher:
nix run /path/to/Nylium#pumpkin

# Build the adapter alone from the checkout:
nix develop -c cargo build --manifest-path nylium-pumpkin/Cargo.toml
```

Run from your server directory. The optional argument selects the directory containing
`pumpkin.toml`; world and player-data paths retain Pumpkin's working-directory semantics.
The console launcher starts the server, prints logs, and saves/shuts it down on Ctrl-C.

The library exposes `PumpkinAdapter`, implementing `nylium_adapter::ServerAdapter`.
Add `adapter.logging_layer()` to your tracing subscriber to capture logs and command feedback.
The optional `ui` feature implements Nylium's UI integration.

Settings edits preserve comments, validate with Pumpkin's configuration types and atomically
replace the configuration. They take effect after restart. External settings edits, online
players and each world's native game rules are sampled automatically; logs are pushed.
Game rule changes are persisted to the world's native rule file. Pumpkin shares its rule registry across dimensions, so editing a rule updates its value in every dimension.

Pumpkin's public profile cache cannot enumerate offline players, so the players page shows
connected players and known operators. This adapter does not install WASM plugin loaders. Non-scalar settings,
telemetry and complex configuration remain editable in TOML.

The Pumpkin revision is pinned in Cargo.toml. The build needs Minecraft 26.3 data; set
`PUMPKIN_MINECRAFT_SERVER_JAR` to the matching server JAR for an offline build. The Nix flake
fetches that JAR with its pinned hash.
