# Nylium

Native Minecraft server administration in Rust, built with [Ely](https://elygpui.com).

The default executable is an **in-memory demonstration**, not a Minecraft server. It includes live logs and commands, process CPU/RAM charts, player administration, game rules, server settings, and a custom adapter page. Configuration resets on application exit. A real server embeds the library and supplies an adapter.

```sh
nix run
nix develop -c cargo test --workspace
nix flake check
```

The flake supplies SteelMC's pinned Rust nightly and the native dependencies; a working graphical session and graphics driver are needed to open the app. Use `nix develop` for the development environment. Ely core and charts are used directly, without the unrelated vertical component crates.

## SteelMC

The `nylium-steel` crate embeds a real SteelMC server. Run it from the directory where you want the server's configuration and worlds:

```sh
nix run /path/to/Nylium#steel
# Optional first argument: path to config/config.toml
nix run /path/to/Nylium#steel -- config/config.toml
```

For development, use `nix develop -c cargo run --manifest-path nylium-steel/Cargo.toml`. The adapter is pinned to a SteelMC revision; its Rust nightly is pinned in `rust-toolchain.toml`. See [the Steel adapter guide](nylium-steel/README.md) for embedding and supported settings.

## Integrating a server

Implement `nylium_adapter::ServerAdapter` for your backend, then implement `nylium::UiAdapter` and call `Nylium::run(adapter)`. `UiAdapter::pages` defaults to no additional pages. There is no GPUI dependency in the backend contract.

- `snapshot()` returns the current authoritative state without I/O. Subscribe **before** reading the initial snapshot.
- Publish an invalidation whenever logs, players, metrics, lifecycle, connection state, field metadata, or field values change. `StatePublisher` provides independent subscriptions with coalesced notifications. Readers always fetch the latest full state; notifications are not a queue of individual changes.
- Async operations return `Result<(), String>`. Validate inputs in the backend, persist settings there, and publish the resulting state **before returning success**. Backend implementations must tolerate concurrent operations on different fields. The UI serializes writes to each field and retains the latest draft while a write is in flight.
- Supply settings and rules as typed `Field` values with stable ids unique within their section. Numeric bounds are inclusive. The rule list is backend-defined so unsupported Minecraft rules do not appear. Changing settings can set `restart_required`; clear it once those settings take effect.
- Include a bounded log history with stable sequence numbers, and bounded metric history. CPU is a percentage of total machine capacity; memory is bytes. The demo retains 1,000 logs and 120 metric samples. Capture your server's logs into this state; stdout is not automatically intercepted.
- Supply UUID, name, online state, dimension and operator state for each known player. Player buttons send `op`, `deop`, `kick` and `ban` commands through the adapter. Avatars are fetched from Mineatar using the UUID, with initials as fallback.
- Start is called when the window opens. Normal window close waits until operations finish, then calls `stop`; failures remain visible. Implement `start` and `stop` idempotently.

See [the demo adapter](nylium/src/demo.rs) for the complete contract in use.

## Adapter pages

Return any GPUI view from `UiAdapter::pages`, wrapped in `AdapterPage::new`. IDs must be nonempty, unique, and outside the reserved `nylium.` prefix. These views sit beside the built-in pages; they are not constrained to settings controls.

```rust,ignore
fn pages(&self, window: &mut Window, cx: &mut App) -> Vec<AdapterPage> {
    let view = cx.new(|cx| MyPage::new(window, cx));
    vec![AdapterPage::new(
        "my-server.worlds", "Worlds", view,
        |page, snapshot, window, cx| page.update_state(snapshot, window, cx),
    )]
}
```

The callback runs initially and for every server invalidation, including while the page is hidden. Nylium notifies the view after the callback. Views with additional backend-specific data can also own their own subscriptions and tasks. Keep callbacks fast; perform I/O asynchronously.

## Settings behavior

The header uses Ely's `SavingIndicator`, `SyncStatus` and `ConnectionStatus`. Valid edits save automatically; writes to the same field are serialized. Invalid drafts and failed writes show an error and are retained, with retry or discard actions. Clean fields follow external changes automatically. Closing is blocked while operations are in progress or drafts have unresolved errors, so an edit is not silently lost.

This cleanroom rewrite replaces the previous `NyliumServer<C, G>` API and `NyliumLogger`; old adapters must migrate to the published-state contract.

## Independent server adapters

The Steel and Pumpkin adapters each have their own Cargo workspace and lockfile.
Their default library/console builds depend on the shared `nylium-adapter` contract,
without building Nylium's UI, Ely or GPUI. Run either from your server directory:

```sh
nix develop /path/to/Nylium -c cargo run --manifest-path /path/to/Nylium/nylium-steel/Cargo.toml -- config/config.toml
nix develop /path/to/Nylium -c cargo run --manifest-path /path/to/Nylium/nylium-pumpkin/Cargo.toml -- .
```

Use Ctrl-C to save and stop. Graphical launchers require `--features ui --bin nylium-steel-ui`
or `--features ui --bin nylium-pumpkin-ui`. Nix supplies `#steel` and `#pumpkin`
for graphical launchers and `#steel-server` / `#pumpkin-server` for console launchers.
Adapter tests are separate from the GUI workspace:

```sh
cargo test --manifest-path nylium-steel/Cargo.toml
cargo test --manifest-path nylium-pumpkin/Cargo.toml
```

See the [Pumpkin adapter guide](nylium-pumpkin/README.md) for its configuration and limits.

`nix flake check` checks Nylium and its shared contract only. The console lifecycle
helper lives in `nylium-adapter::console`; both server launchers use it, including
Ctrl-C handling during startup. Test it with
`cargo test -p nylium-adapter --features console`.
