# Entity CLI — LLM‑First Global Graph Engine

## Overview
Entity CLI is an engine that boots into a machine‑native session where the model sees the entire capability graph at all times. No discovery commands. The model picks any visible node and issues a single intentful command with one‑shot selections.

## Core ideas
- Always‑visible graph: the full set of capabilities and metadata is emitted at session start and remains visible.
- Jump by intent: the command targets the node already visible; no implicit prerequisite resolution.
- Node kinds:
  - Docs (read): return content from the pack.
  - Components (write): copy source trees into the user’s workspace.
  - Setup (scaffold): run product-authored scaffolds and copy templates.
  - Bridge (replicator): manage real-time database bridge templates and runners.

## Quick start
Using the npm shim that downloads the right native binary:

```bash
npx entity-cli init entity-auth
npx entity-cli docs read entity-auth --node entityauth:docs:getting-started
npx entity-cli ui install entity-auth --mode all
```

Notes:
- Product is positional (e.g., `entity-auth`, `microsoft`).
- Install target is the current directory; to install elsewhere, `cd` first.

## Behavior
- No discovery/help: the graph is emitted once; the agent already has context.
- One‑shot selections: missing prerequisites produce a single JSON error with required keys.
- Deterministic layout: components are written under `entity-auth/components/<Name>/` with overwrite‑on‑write semantics.

## CLI commands
- Initialize session (emit graph):
  - `entity-cli init <product>`
- Docs:
  - `entity-cli docs read <product> --node <docId>`
- Components:
  - `entity-cli ui install <product> --mode <single|multiple|all> [--names <Name...>]`
- Setup:
  - `entity-cli setup run <product> --node <setupId> [--workspace <path>]`
- Bridge:
  - `entity-cli bridge scaffold <product> --node <bridgeId> [--workspace <path>]`
  - `entity-cli bridge start <product> --node <bridgeId> [--workspace <path>]`
  - `entity-cli bridge attach <product> --node <bridgeId> --pid <pid> [--status <label>] [--status-message <text>] [--workspace <path>]`
  - `entity-cli bridge heartbeat <product> --node <bridgeId> [--status <label>] [--status-message <text>] [--workspace <path>]`
  - `entity-cli bridge status <product> --node <bridgeId> [--workspace <path>]`
  - `entity-cli bridge stop <product> --node <bridgeId> [--workspace <path>]`

### Bridge workflow

- `bridge scaffold` copies the template tree (`bridge/templates/<name>`) into the workspace under `entity-auth/bridge/<name>`.
- `bridge start` resolves the runner file (`runner.mjs` or spawn descriptor), generates a process JSON payload, and persists it to `.entitycli/bridge/state/<node>.json`. This JSON includes env defaults, arguments, config/log paths, and a freshly generated `stateId`.
- Supervisors spawn the worker (typically a Node replicator), then call `bridge attach` with the child PID (and optional status message). This updates the persisted state so `status` reflects the running process.
- Workers should periodically call `bridge heartbeat` (or emit heartbeats via the runtime hooks) to refresh status/health metadata.
- `bridge status` reads the state file and exposes the latest PID, heartbeat timestamp, logs path, etc.
- `bridge stop` signals the persisted state as stopped, sends `SIGINT` to the tracked PID on unix hosts, and removes the state file after the stop command completes.

## Errors (JSON envelope)
- `UNKNOWN_NODE`, `WRONG_KIND`, `MISSING_SELECTIONS`, `INVALID_SELECTION`, `INVALID_SELECTION` (names), `PACKS_NOT_FOUND`, `TARGET_NOT_FOUND`, `TARGET_NOT_WRITABLE`.

## Pack authoring (example: Entity Auth)
- `packs/entity-auth/docs/nodes.json`
- `packs/entity-auth/docs/content/*.md`
- `packs/entity-auth/components/nodes.json`
- `packs/entity-auth/components/ui/<Name>/...`
- `packs/entity-auth/setup/nodes.json`
- `packs/entity-auth/setup/templates/<templateName>/entity-auth/{client.ts,provider.tsx,middleware.ts,components/...}`

## Setup nodes

Setup is a first-class node kind that lets products define end-to-end app initialization in one command.

- What the engine does:
  - Executes optional, non-interactive scaffold commands (e.g., `create-next-app`).
  - Copies the product-authored template tree into `workspace/entity-auth`.
  - Emits a JSON report with executed commands and copied paths.

- How products define setup:
  - Add a `setup/nodes.json` file with one or more setup nodes. Each node’s payload includes:
    - `templateRoot`: path to the inner directory whose contents should land directly under `entity-auth`.
    - `commands`: array of shell commands to run before copying (optional).
  - Place template files under `setup/templates/<name>/entity-auth/...`.

- How consumers run it:
  - Generic engine: `entity-cli setup run <product> --node <setupId> [--workspace <path>]`
  - Product shim (bundled packs): `npx @<product>/cli setup run <product> --node <setupId>`

Example (Entity Auth):

```bash
# Emit graph (shim passes packs automatically)
npx @entityauth/cli init entity-auth

# Scaffold Next.js + install baseline files under ./entity-auth
npx @entityauth/cli setup run entity-auth --node entityauth:setup:basic
```

Best practices:
- Make scaffold commands non-interactive and idempotent.
- Point `templateRoot` to the inner `entity-auth` directory to avoid `entity-auth/entity-auth`.
- Provide multiple templates (e.g., basic, with-db) with descriptive `meta.tags`.

## Distribution
- Prebuilt binaries attached to GitHub Releases.
- npm package `entity-cli` fetches and spawns the native binary.
- Optional product shims (e.g., `@entityauth/cli`) pre‑select a product for zero‑arg affordance.

## Platform support (current)
- macOS Apple Silicon (arm64) only.
- Intel macOS, Windows, and Linux are intentionally not supported yet. This is deliberate to keep distribution simple during the MVP. We’ll expand support later.


## Test coverage

We use cargo-llvm-cov for workspace coverage.

Prereqs:

```bash
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov --locked
```

Run HTML report:

```bash
make coverage
# or
cargo coverage
open target/llvm-cov/html/index.html
```

Export LCOV (for CI/Codecov):

```bash
make coverage-lcov
# or
cargo coverage-lcov
```

Clean coverage data:

```bash
make coverage-clean
```

CI publishes `Entity-CLI/coverage/lcov.info` as an artifact on PRs and pushes to `main`.

