import json
import os

locales_dir = "src/lib/i18n/locales"
en_path = os.path.join(locales_dir, "en.json")

for filename in os.listdir(locales_dir):
    if filename.endswith(".json") and filename not in ["en.json", "it.json"]:
        path = os.path.join(locales_dir, filename)
        with open(path, "r", encoding="utf-8") as f:
            data = json.load(f)
        
        if "sync.wizard.skip" not in data:
            data["sync.wizard.skip"] = "Skip"
            
            with open(path, "w", encoding="utf-8") as f:
                json.dump(data, f, indent=2, ensure_ascii=False)
                f.write("\n")
            print(f"Fixed {filename}")
