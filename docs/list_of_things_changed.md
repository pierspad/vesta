# Modifiche recenti (in preparazione alla release)

## [Data Odierna] - Internazionalizzazione (i18n) e Revision Tab
- **Localizzazione tab Revisione**: Sostituite tutte le 20 stringhe hardcoded all'interno di `AlignTab.svelte` con chiamate all'helper di traduzione `t(key)`.
- **Integrazione chiavi JSON**: Aggiunte le chiavi in lingua originale nel file locale `en.json` (da `align.baseSrt` a `align.noSubtitle`).
- **Traduzione automatizzata multi-lingua**: Aggiunte e generate le traduzioni per tutte le 20 nuove chiavi relative alla tab "Revision" in altre 12 lingue (`ar`, `de`, `es`, `fr`, `hi`, `it`, `ja`, `ko`, `pl`, `pt`, `ru`, `tr`, `zh`), risolvendo i fault rilevati dallo script di sanity check `check_missing_translations.py`.
- **Fix di validazione francese**: Bypassato provvisoriamente il controllo di uguaglianza con l'inglese per la chiave `align.page` (la traduzione in francese era identica alla parola in inglese "Page").
