# Vesta Documentation

Vesta decouples reusable Rust feature engines from its desktop, CLI, and MCP consumers.
This index links to all actively maintained reference manuals and guides.

## User & CLI/MCP Guides

| Guide | Description |
| --- | --- |
| [CLI and Coverage](CLI.md) | Commands, examples, safe usage, error handling, output formats, and differences from GUI/libraries. |
| [MCP Server](MCP.md) | Local stdio server, 10 tools, allowed directory roots, provider profiles, and asynchronous job management. |
| [Plugin Specification & SDK](PLUGINS.md) | Declarative JSON-RPC 2.0 plugin architecture, widget catalog, manifest schema, and Python SDK. |
| [Setup and Installer Testing](SETUP_TESTING.md) | First-run setup checks, audio preparation, and installer update validation. |

## Architecture & Module Reference

| Guide | Description |
| --- | --- |
| [Architecture](ARCHITECTURE.md) | Module boundaries, desktop integration, and media lifecycles. |
| [Modules](modules/README.md) | Engine APIs and standalone CLI tools. |
| [Library Integration](library-integration.md) | Dependencies, feature flags, and validation for external consumers. |
| [Provider Compatibility](API_COMPATIBILITY.md) | Public HTTP contracts and endpoint references for cloud STT and LLM integrations. |
| [Maintenance](MAINTENANCE.md) | Architectural boundaries, persistence contracts, and dependency advisories. |

## Quality & Benchmarks

| Guide | Description |
| --- | --- |
| [Quality & Release](QUALITY.md) | Automated tests, smoke checks, and installed product verification. |
| [Reproducing Benchmarks](BENCHMARK_STEPS.md) | Step-by-step procedures, scripts, and measurement caveats. |
| [Native Benchmarks](BENCHMARK_NATIVE.md) | Comparative evaluations with output equivalence checks. |
| [Benchmark Report](BENCHMARK_REPORT.md) | Context and published performance measurements. |

A passing build or green test suite does not certify installers, real GPUs, Anki connectivity, or remote providers. Automated checks and real-world verifications are distinguished in [QUALITY](QUALITY.md).
