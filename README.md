# Entity CLI — LLM‑First Global Graph Engine

## Overview
Entity CLI is an engine that boots into a machine‑native session where the model sees the entire capability graph at all times. No discovery commands. The model picks any visible node and issues a single intentful command with one‑shot selections.

## Core ideas
- Always‑visible graph: the full set of capabilities and metadata is emitted at session start and remains visible.
- Jump by intent: the command targets the node already visible; no implicit prerequisite resolution.
- Node kinds (MVP):
  - Docs (read): return content from the pack.
  - Components (write): copy source trees into the user’s workspace.

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
- Docs:
  - `entity-cli docs read <product> --node <docId>`
- Components:
  - `entity-cli ui install <product> --mode <single|multiple|all> [--names <Name...>]`
- Initialize session (emit graph):
  - `entity-cli init <product>`

## Errors (JSON envelope)
- `UNKNOWN_NODE`, `WRONG_KIND`, `MISSING_SELECTIONS`, `INVALID_SELECTION`, `INVALID_SELECTION` (names), `PACKS_NOT_FOUND`, `TARGET_NOT_FOUND`, `TARGET_NOT_WRITABLE`.

## Pack authoring (example: Entity Auth)
- `packs/entity-auth/docs/nodes.json`
- `packs/entity-auth/docs/content/*.md`
- `packs/entity-auth/components/nodes.json`
- `packs/entity-auth/components/ui/<Name>/...`

## Distribution
- Prebuilt binaries attached to GitHub Releases.
- npm package `entity-cli` fetches and spawns the native binary.
- Optional product shims (e.g., `@entityauth/cli`) pre‑select a product for zero‑arg affordance.

## Platform support (current)
- macOS Apple Silicon (arm64) only.
- Intel macOS, Windows, and Linux are intentionally not supported yet. This is deliberate to keep distribution simple during the MVP. We’ll expand support later.


