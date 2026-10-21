# Documentazione Vesta

Vesta separa i motori Rust riutilizzabili dai consumatori desktop, CLI e MCP.
Questo indice rimanda alle guide mantenute. Le revisioni del codice sono
snapshot separati: [audit del 21 ottobre](../reports/CODEBASE_AUDIT_2026-10-21.md) e
[interventi del 22 ottobre](../reports/CODEBASE_AUDIT_FOLLOWUP_2026-10-22.md), con la
[revisione di documentazione e CLI](../reports/DOCUMENTATION_CLI_REVIEW_2026-10-22.md).

| Guida | Contenuto |
| --- | --- |
| [Architettura](ARCHITECTURE.md) | Confini dei moduli, integrazione desktop e ciclo di vita dei media. |
| [CLI e copertura](CLI.md) | Comandi, esempi, errori d’uso, output e differenze rispetto a GUI/librerie. |
| [Moduli](modules/README.md) | Uso dei motori e delle CLI. |
| [Integrazione librerie](library-integration.md) | Dipendenze, feature e verifica dei consumer esterni. |
| [Compatibilità provider](API_COMPATIBILITY.md) | Revisione datata dei contratti API e dei link dei provider; non è una policy di compatibilità delle librerie Rust. |
| [MCP](MCP.md) | Server locale, strumenti, directory consentite e gestione dei job. |
| [Qualità e release](QUALITY.md) | Test, smoke e verifiche del prodotto installato. |
| [Setup e installer](SETUP_TESTING.md) | Prova del setup, audio e installazione. |
| [Manutenzione](MAINTENANCE.md) | Correzioni consolidate, limiti e lavoro ancora aperto. |
| [Riprodurre i benchmark](BENCHMARK_STEPS.md) | Procedure e limiti delle misure. |
| [Benchmark nativo](BENCHMARK_NATIVE.md) | Confronti con verifica dell’equivalenza degli output. |
| [Report benchmark](BENCHMARK_REPORT.md) | Risultati e contesto delle misure pubblicate. |

Una build o una suite verde non certificano installer, GPU, Anki e provider
remoti. Le verifiche automatizzate e quelle reali sono distinte in QUALITY.
