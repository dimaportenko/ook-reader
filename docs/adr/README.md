# Architecture Decision Records

Short, dated records of decisions that shape the project — the *why* behind a choice, so we
don't relitigate it later (or so we know what we're overturning when we do). Written as
decisions land, often out of a grilling session. Format: Context → Decision → Consequences.

| # | Decision | Status |
|---|----------|--------|
| [0001](0001-walking-skeleton-vertical-slices.md) | Build the reader as thin vertical slices (walking skeleton) | accepted |
| [0002](0002-dogfood-driven-prioritization.md) | Dogfood-driven prioritization — usage pain orders the backlog | accepted |
| [0003](0003-reader-controlled-theming-injected-layer.md) | Reader-controlled theming via an injected override layer (layer, don't replace; serve XHTML; no `rbook` fork) | accepted |
| [0004](0004-local-store-rusqlite-with-libsql-sync-path.md) | Local store on `rusqlite` now, with a libSQL/Turso sync path later | **superseded** by 0005 & 0006 |
| [0005](0005-backendless-sync-google-drive-per-device-shards.md) | Backend-less sync: Google Drive `appDataFolder` + per-device snapshot shards (no server; merge, not arbitrate) | accepted |
| [0006](0006-migrate-local-store-rusqlite-to-sqlx.md) | Migrate the local store from `rusqlite` to `sqlx` (gradually, via a `rusqlite` 0.39 transition) | accepted |
