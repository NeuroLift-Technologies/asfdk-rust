# Namespace Overview

This repository (`NeuroLift-Technologies/asfdk-rust`) is the Rust port of the NeuroLift Technologies ASFDK (Solidarity Framework Development Kit).

**Purpose:** Provide TOI/OTOI/ASFDK governance compliance for Rust ecosystems, enabling:
- Agent registration and governance boundary definition
- Terms of Interaction (TOI) enforcement
- Terms of Operation (OTOI) compliance checking
- ASFDK integration points for Rust services and agents

**Related repositories:**
- `NeuroLift-Technologies/asfdk` — Original ASFDK (Python/TypeScript reference)
- `NeuroLift-Technologies/asfdk-go` — Go port
- `NeuroLift-Technologies/asfdk-csharp` — C#/.NET port
- `NeuroLift-Technologies/asfdk-kotlin` — Kotlin port
- `NeuroLift-Technologies/asfdk-cplus` — C++ port
- `NeuroLift-Technologies/.github-private` — Org-level governance

**Current status:** Core governance framework ported (TOI/OTOI validation, prompt defense, Sleepwalker, RRT, unified foundation). Governance validation with Rust toolchain gates.

**Key directories:**
- `.nltotoi/` — Discovery manifest and governance file registry
- `templates/` — OTOI Section 3 registration format, handoff records, escalation format, intent logging
- `src/` — Rust implementation (lib crate `asfdk`)
- `docs/` — Governance and port documentation
