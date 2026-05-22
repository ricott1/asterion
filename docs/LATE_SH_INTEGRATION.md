# Integrating asterion-core into late.sh

`asterion-core` was extracted from the asterion bin so it can be embedded as
a multiplayer room game inside [late.sh](https://github.com/mpiorowski/late-sh)
without dragging in asterion's own SSH/Tui layer. This doc captures the
integration recipe so the PR can be opened later without rederiving the
design.

## Why this works

- **License**: late.sh is FSL-1.1-MIT. The asterion bin is GPL-3.0-or-later
  (incompatible with FSL). `asterion-core` is dual-licensed MIT OR Apache-2.0
  so late.sh can depend on it. Only the lib is consumable - the bin is not.
- **Architecture fit**: late.sh games follow a Service/State split (one
  `Service` per room owns the truth, sessions subscribe via `tokio::sync::watch`
  to snapshots). `asterion_core::Game` is the truth; a `SharedState` wrapper
  exposes the right methods. All players in a room see the same maze snapshot;
  no asymmetric-info channel is needed.

## late.sh's room model in 30 seconds

Each game lives at `late-ssh/src/app/rooms/<game>/` and implements three traits
from `late-ssh/src/app/rooms/backend.rs`:

- `RoomGameManager` (process-wide; one instance, registered in
  `RoomGameRegistry`).
- `ActiveRoomBackend` (per-session, held on `App.active_room_game`).
- `CreateRoomModal` (owns the room-creation form UI).

Canonical reference implementation: `late-ssh/src/app/rooms/tictactoe/svc.rs`
(~200 lines, covers Service/SharedState split, watch fanout, defensive
validation under lock).

The repo's `GAMES.md` is the contributor guide. Read it before opening the
PR.

## File layout in the late.sh PR

```
late-ssh/src/app/rooms/asterion/
  mod.rs           pub mod declarations only (no re-exports)
  manager.rs       impl RoomGameManager + impl ActiveRoomBackend for State
  svc.rs           AsterionService { snapshot_tx, snapshot_rx, state: Arc<Mutex<SharedState>> }
                   SharedState wraps asterion_core::Game
                   *_task methods + a tick_task that drives Game::update()
  state.rs         per-session: cursor + cached AsterionSnapshot + watch::Receiver
  input.rs         byte → KeyCode → asterion_core::GameCommand → state methods
                   (use the key-mapping from asterion's own bin as a starting point)
  ui.rs            draw_game(frame, area, &state, usernames) using late.sh theme::*
  create_modal.rs  room name + maze size + difficulty form
  settings.rs      AsterionSettings { maze_size, difficulty } + from_json/to_json/normalized
```

## Glue contract with asterion-core

- `SharedState` holds an `asterion_core::Game`.
- Service methods follow late.sh's `*_task` pattern:
  - spawn a tokio task
  - `state.lock().await`
  - call `Game::handle_command(cmd, player_id)` (or whichever mutating method
    applies)
  - call a local `publish(&state)` that drops `snapshot_tx.send(...)`.
- A background **`tick_task`** spawned at service construction calls
  `Game::update()` on a `tokio::time::interval(Game::update_time_step())` and
  publishes after each tick. This is what makes the minotaur move without
  user input.
- All players see the same snapshot. The poker-style per-user private channel
  pattern is **not** needed for asterion - personalized hero highlighting is a
  render-layer concern (see below).

## Late.sh username → asterion hero name

`asterion_core::Game::add_player(player_id, username)` accepts a username
string. In the late.sh `manager.rs` glue, look up the late.sh username for the
incoming `user_id` (the rooms layer exposes the lookup via a registry or
through `App` state - confirm by reading `tictactoe/manager.rs` first) and
pass it to `add_player` when the session enters the room.

This is the load-bearing reason `add_player`'s signature must keep
`username: String`. Do not drop it from the lib API.

## Per-session hero highlighting (not asymmetric info)

In `ui::draw_game(frame, area, &state, usernames)`, iterate the snapshot's
heroes and color the one matching `state.user_id` distinctly. The other heroes
are drawn with a different color. This is just per-session rendering of a
shared snapshot, not asymmetric-info - all positions are public, the maze
itself is identical for everyone.

If asterion later grows fog-of-war (visibility limited to your hero's
vicinity), that would become asymmetric-info and need the Poker-style
split-channel pattern.

## Key conventions

- `Esc` and `q` return `InputAction::Leave`.
- Avoid `j`/`k`/arrows/`i`/`d`/`r`/`e`/`p`/`c`/`f`/`g` - these are reserved
  for chat by late.sh's `should_route_active_room_chat_key` heuristic. Use
  WASD for movement. (See `rooms/input.rs` in late.sh for the full list.)

## Theme

Use `theme::*` from `late-ssh/src/app/common/theme.rs`. Do not hardcode
colors. Provide a small-area fallback layout - the rooms layer can shrink
the game pane when chat takes priority.

## Game lifecycle

When heroes escape or all are caught, the snapshot enters a "game over"
state. v1 recommendation: auto-restart with a fresh maze after a short delay
(~30s). Decide based on what feels right during the actual PR.

## Process for the PR

1. **Open an issue first** asking the maintainer (mpiorowski) whether they
   prefer a crate dependency or vendored code for game contributions. If
   they push back on the dep, vendor the asterion-core source files
   instead. Don't do the work in a PR until this is settled.
2. Fork `mpiorowski/late-sh`.
3. Add `GameKind::Asterion` in `late-core/src/models/game_room.rs`
   (variant + `as_str = "asterion"` + add to the `ALL` array + `parse`
   arm).
4. Add `asterion-core` dep to `late-ssh/Cargo.toml`.
5. Create the 7 files in `late-ssh/src/app/rooms/asterion/`.
6. Wire into `RoomGameRegistry::new` (`late-ssh/src/app/rooms/registry.rs`):
   add a field, accept it in `new()`, add a match arm in `manager()`.
7. Construct the manager in `late-ssh/src/main.rs` and pass to the registry.
8. Add a filter-label arm in `late-ssh/src/app/rooms/filter.rs`.
9. Append a short section to `late-ssh/src/app/rooms/CONTEXT.md` covering
   the runtime model (real-time tick), keys, seats, and invariants.
10. **DCO-sign every commit** (`git commit -s`).
11. **No em dashes** in copy or commit messages.
12. **`Uuid::now_v7()`** not `Uuid::new_v4()` for any new IDs (late.sh
    convention).
13. `make check` clean before opening the PR.

## What this crate must keep providing

These are the load-bearing pieces of the public API. Don't break them
without coordinating with the late.sh integration:

- `Game::add_player(player_id: PlayerId, username: String)` - keep the
  username arg.
- `Game::remove_player(&PlayerId)`.
- `Game::handle_command(&GameCommand, PlayerId)`.
- `Game::update()`.
- `Game::update_time_step() -> Duration` - the cadence the late.sh
  background tick task uses.
- A snapshot/view accessor reachable from outside the crate so callers can
  render without grabbing private internals.
- `Game: Send` so the late.sh service can hold it in `Arc<Mutex<...>>`
  across `.await` points.

## Reference reading inside late.sh

- `GAMES.md` at the repo root - the contributor guide.
- `late-ssh/src/app/rooms/tictactoe/svc.rs` - canonical ~200-line example.
- `late-ssh/src/app/rooms/blackjack/svc.rs` - shows the AFK background
  task pattern that the asterion tick task will mirror.
- `late-ssh/src/app/rooms/CONTEXT.md` - internal architecture notes.
