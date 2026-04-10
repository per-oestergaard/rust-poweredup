---
name: cargo-pin
description: >
  Pin all Cargo.toml dependencies to their current latest exact versions, and update
  pinned versions to the latest available. Use when: auditing dependency versions,
  pinning floating version constraints (e.g. "1" → "1.2.3"), updating pinned deps,
  supply-chain hardening for Rust crates.
---

# Cargo Pin

Pins every dependency in every `Cargo.toml` in the workspace to an exact version
(`=X.Y.Z`). Preserves all other fields. `path` and `git` deps are never touched.

**Before:**

```toml
tokio = { version = "1", features = ["full"] }
bytes = "1"
```

**After:**

```toml
tokio = { version = "=1.44.0", features = ["full"] }
bytes = "=1.10.1"
```

## When to Use

- Pinning floating version constraints for supply-chain hardening
- Updating already-pinned deps to the latest exact release
- Any request to "pin cargo deps", "update pinned versions", or `/cargo-pin`

## Procedure

This is an **agent-driven skill** — no external script. Perform each step yourself
using the available tools.

### 1. Read all Cargo.toml files

Read the workspace root `Cargo.toml` and every member `Cargo.toml`. Collect every
dependency entry under `[dependencies]`, `[dev-dependencies]`, `[build-dependencies]`,
and `[target.*.dependencies]` sections.

Skip entries that contain `path =`, `git =`, or `workspace = true`.

### 2. Resolve latest versions

For each unique crate name, run in the terminal:

```
cargo search <crate-name> --limit 1
```

Parse the first output line — it looks like:

```
crate-name = "X.Y.Z"    # description
```

Extract `X.Y.Z` as the latest version.

### 3. Edit the files

For each dependency where the current spec differs from `=X.Y.Z`:

- Inline form `"1"` → `"=X.Y.Z"`
- Table form `version = "1"` → `version = "=X.Y.Z"`
- If already `"=X.Y.Z"` and up to date, skip.

Edit `Cargo.toml` directly using the file edit tool. Preserve all other fields
(`features`, `optional`, etc.) exactly.

### 4. Regenerate the lock file and verify

```
cargo update
cargo test -p poweredup
```

### 5. Report

Emit a summary table of every change:

| Crate | Before | After       |
| ----- | ------ | ----------- |
| tokio | `"1"`  | `"=1.44.0"` |
| bytes | `"1"`  | `"=1.10.1"` |

## Rules

- `path`, `git`, and `workspace = true` deps: **never modify**.
- Preserve `features`, `optional`, and all other fields exactly.
- Do not modify `rust-toolchain.toml`.
- If `cargo search` returns no result for a crate, leave it unchanged and report it.
