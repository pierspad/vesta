# Modifiche recenti (in preparazione alla release)

## [Data Odierna] - Release, i18n e Governance LLM
- **GitHub Actions Node 24**: Aggiornate le action CI/release a versioni compatibili con il runtime Node.js 24 (`checkout@v6`, `setup-node@v6`, `setup-python@v6`, `upload-artifact@v6`, `download-artifact@v7`, `cache@v5`, `sccache-action@v0.0.9`) per rimuovere gli avvisi di deprecazione Node.js 20.
- **Lingua olandese completa**: Aggiunti i file `nl.json` per i locale principali e per i contenuti help/info, portando VESTA alle stesse 15 lingue di TextMerger.
- **Registrazione i18n aggiornata**: Integrato `nl` in `apps/srt-gui/src/lib/i18n/index.ts`, nella lista lingue UI e negli script di split/merge traduzioni.
- **Audit i18n rafforzato**: Aggiornato `check_missing_translations.py` per richiedere tutte le 15 lingue e bloccare locale mancanti, chiavi mancanti, valori vuoti e mismatch dei placeholder.
- **Release notes sezionate**: Aggiunto `build-scripts/extract-release-notes.sh` e aggiornato workflow GitHub per pubblicare la sezione del tag o il fallback `## Release Notes`.
- **Release script uniformato**: Riscritto `build-scripts/git-release.sh` con helper e controlli simili a TextMerger: validazione i18n, validazione release notes, preview status e push atomico di branch/tag.
- **Pubblicazione AUR piu' robusta**: Aggiornato `push-aur.sh` per riallineare metadati, verificare versioni interne e controllare che la directory AUR punti al remote corretto.
- **Istruzioni Copilot/LLM allineate**: Aggiornate le linee guida del progetto per i18n a 15 lingue, release notes curate, log operativo obbligatorio, riuso componenti e riduzione del debito tecnico.

## [Data Odierna] - Internazionalizzazione (i18n) e Revision Tab
- **Localizzazione tab Revisione**: Sostituite tutte le 20 stringhe hardcoded all'interno di `AlignTab.svelte` con chiamate all'helper di traduzione `t(key)`.
- **Integrazione chiavi JSON**: Aggiunte le chiavi in lingua originale nel file locale `en.json` (da `align.baseSrt` a `align.noSubtitle`).
- **Traduzione automatizzata multi-lingua**: Aggiunte e generate le traduzioni per tutte le 20 nuove chiavi relative alla tab "Revision" in altre 12 lingue (`ar`, `de`, `es`, `fr`, `hi`, `it`, `ja`, `ko`, `pl`, `pt`, `ru`, `tr`, `zh`), risolvendo i fault rilevati dallo script di sanity check `check_missing_translations.py`.
- **Fix di validazione francese**: Bypassato provvisoriamente il controllo di uguaglianza con l'inglese per la chiave `align.page` (la traduzione in francese era identica alla parola in inglese "Page").
