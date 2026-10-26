# Vesta Documentation

Vesta decouples reusable Rust feature engines from its desktop, CLI, and MCP consumers.
This index links to all actively maintained guides. Historical code audits are preserved
as standalone snapshots: [October 21 audit](../reports/CODEBASE_AUDIT_2026-10-21.md),
[October 22 follow-up](../reports/CODEBASE_AUDIT_FOLLOWUP_2026-10-22.md), and the
[documentation and CLI review](../reports/DOCUMENTATION_CLI_REVIEW_2026-10-22.md).

## User & CLI/MCP Guides

| Guide | Description |
| --- | --- |
| [CLI and Coverage](CLI.md) | Commands, examples, safe usage, error handling, output formats, and differences from GUI/libraries. |
| [MCP Server](MCP.md) | Local stdio server, tools, allowed directory roots, and asynchronous job management. |
| [Setup and Installer Testing](SETUP_TESTING.md) | First-run setup checks, audio preparation, and installer update validation. |

## Architecture & Module Reference

| Guide | Description |
| --- | --- |
| [Architecture](ARCHITECTURE.md) | Module boundaries, desktop integration, and media lifecycles. |
| [Modules](modules/README.md) | Engine APIs and standalone CLI tools. |
| [Library Integration](library-integration.md) | Dependencies, feature flags, and validation for external consumers. |
| [Provider Compatibility](API_COMPATIBILITY.md) | Historical review of API contracts and provider endpoints (not a Rust library compatibility policy). |
| [Maintenance](MAINTENANCE.md) | Architectural boundaries, persistence contracts, and open tasks. |

## Quality & Benchmarks

| Guide | Description |
| --- | --- |
| [Quality & Release](QUALITY.md) | Automated tests, smoke checks, and installed product verification. |
| [Reproducing Benchmarks](BENCHMARK_STEPS.md) | Step-by-step procedures, scripts, and measurement caveats. |
| [Native Benchmarks](BENCHMARK_NATIVE.md) | Comparative evaluations with output equivalence checks. |
| [Benchmark Report](BENCHMARK_REPORT.md) | Context and published performance measurements. |

A passing build or green test suite does not certify installers, real GPUs, Anki connectivity, or remote providers. Automated checks and real-world verifications are distinguished in [QUALITY](QUALITY.md).
