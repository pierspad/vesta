import json
import os

locales_dir = 'apps/srt-gui/src/lib/i18n/locales'
translations = {
    "en": {"shortcuts.action.autoSync": "Auto-Sync", "shortcuts.action.newSync": "New Sync", "shortcuts.action.loadSession": "Load Session", "shortcuts.action.saveSession": "Save Session"},
    "it": {"shortcuts.action.autoSync": "Avvia Auto-Sync", "shortcuts.action.newSync": "Nuova Sincronizzazione", "shortcuts.action.loadSession": "Carica Sessione", "shortcuts.action.saveSession": "Salva Sessione"},
}

for filename in os.listdir(locales_dir):
    if not filename.endswith('.json'):
        continue
    filepath = os.path.join(locales_dir, filename)
    with open(filepath, 'r+', encoding='utf-8') as f:
        data = json.load(f)
        lang = filename.split('.')[0]
        
        # fallback to english if locale not explicitly provided above
        updates = translations.get(lang, translations["en"])
        
        for k, v in updates.items():
            if k not in data:
                data[k] = v
                
        f.seek(0)
        json.dump(data, f, ensure_ascii=False, indent=2)
        f.truncate()

print("Translations updated.")
