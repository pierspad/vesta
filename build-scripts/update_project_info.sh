#!/bin/bash
# ===========================================================================
# update_project_info.sh
# Legge le informazioni dal PKGBUILD (Single Source of Truth) e le propaga
# in tutto il progetto: tauri.conf.json, Cargo.toml, package.json, .desktop,
# flatpak manifest, ecc.
# ===========================================================================

set -euo pipefail
export LC_ALL=C

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# ── Rileva root del progetto ──────────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

PKGBUILD="$SCRIPT_DIR/PKGBUILD"
if [ ! -f "$PKGBUILD" ]; then
    echo -e "${RED}❌ PKGBUILD non trovato in $SCRIPT_DIR${NC}"
    exit 1
fi

# ── Leggi campi dal PKGBUILD ──────────────────────────────────
read_pkgbuild() {
    grep -Po "^${1}=\\K.*" "$PKGBUILD" | tr -d "'" | tr -d '"'
}

VERSION=$(read_pkgbuild "pkgver")
PKGDESC=$(read_pkgbuild "pkgdesc")
URL=$(read_pkgbuild "url")
LICENSE=$(read_pkgbuild "license" | tr -d '()')
APP_NAME="vesta"
APP_ID="com.vesta.desktop"
BINARY_NAME="vesta"

echo -e "${BLUE}══════════════════════════════════════════════${NC}"
echo -e "${BLUE}  vesta - Update Project Info${NC}"
echo -e "${BLUE}══════════════════════════════════════════════${NC}"
echo -e "  Versione:     ${GREEN}${VERSION}${NC}"
echo -e "  Descrizione:  ${GREEN}${PKGDESC}${NC}"
echo -e "  URL:          ${GREEN}${URL}${NC}"
echo -e "  Licenza:      ${GREEN}${LICENSE}${NC}"
echo -e "${BLUE}══════════════════════════════════════════════${NC}"
echo ""

ERRORS=0

# ── Helper: aggiorna un file e logga il risultato ─────────────
update_file() {
    local file="$1"
    local description="$2"
    local relative="${file#$PROJECT_ROOT/}"

    if [ ! -f "$file" ]; then
        echo -e "  ${YELLOW}⚠ $relative non trovato, skip${NC}"
        return
    fi
    echo -e "  ${GREEN}✓${NC} $relative — $description"
}

# ===========================================================
# 1. tauri.conf.json
# ===========================================================
TAURI_CONF="$PROJECT_ROOT/apps/srt-gui/src-tauri/tauri.conf.json"
if [ -f "$TAURI_CONF" ]; then
    # Aggiorna productName
    sed -i "s/\"productName\": \".*\"/\"productName\": \"${APP_NAME}\"/" "$TAURI_CONF"
    # Aggiorna version
    sed -i "s/\"version\": \".*\"/\"version\": \"${VERSION}\"/" "$TAURI_CONF"
    # Aggiorna identifier
    sed -i "s/\"identifier\": \".*\"/\"identifier\": \"${APP_ID}\"/" "$TAURI_CONF"
    update_file "$TAURI_CONF" "productName, version, identifier"
else
    echo -e "  ${RED}✗ tauri.conf.json non trovato${NC}"
    ((ERRORS++))
fi

# ===========================================================
# 2. src-tauri/Cargo.toml
# ===========================================================
TAURI_CARGO="$PROJECT_ROOT/apps/srt-gui/src-tauri/Cargo.toml"
if [ -f "$TAURI_CARGO" ]; then
    SAFE_PKGDESC="${PKGDESC//&/\\&}"
    SAFE_PKGDESC="${SAFE_PKGDESC//\//\\/}"
    sed -i "s/^version = \".*\"/version = \"${VERSION}\"/" "$TAURI_CARGO"
    sed -i "s/^description = \".*\"/description = \"${SAFE_PKGDESC}\"/" "$TAURI_CARGO"
    sed -i "s/^license = \".*\"/license = \"${LICENSE}\"/" "$TAURI_CARGO"
    update_file "$TAURI_CARGO" "version, description, license"
else
    echo -e "  ${RED}✗ src-tauri/Cargo.toml non trovato${NC}"
    ((ERRORS++))
fi

# ===========================================================
# 3. package.json (frontend)
# ===========================================================
PKG_JSON="$PROJECT_ROOT/apps/srt-gui/package.json"
if [ -f "$PKG_JSON" ]; then
    # Usa un approccio che funziona senza jq
    sed -i "s/\"version\": \".*\"/\"version\": \"${VERSION}\"/" "$PKG_JSON"
    update_file "$PKG_JSON" "version"
else
    echo -e "  ${RED}✗ package.json non trovato${NC}"
    ((ERRORS++))
fi

# ===========================================================
# 4. .desktop file (build-scripts)
# ===========================================================
DESKTOP_FILE="$SCRIPT_DIR/com.vesta.desktop.desktop"
if [ -f "$DESKTOP_FILE" ]; then
    cat > "$DESKTOP_FILE" <<EOF
[Desktop Entry]
Name=${APP_NAME}
Comment=${PKGDESC}
Exec=${BINARY_NAME}
Icon=${BINARY_NAME}
Type=Application
Categories=AudioVideo;Education;Utility;
Terminal=false
StartupWMClass=${BINARY_NAME}
EOF
    update_file "$DESKTOP_FILE" "name, comment, exec, icon"
fi

DESKTOP_FILE2="$SCRIPT_DIR/vesta.desktop"
if [ -f "$DESKTOP_FILE2" ]; then
    cat > "$DESKTOP_FILE2" <<EOF
[Desktop Entry]
Name=${APP_NAME}
Comment=${PKGDESC}
Exec=${BINARY_NAME}
Icon=${BINARY_NAME}
Type=Application
Categories=AudioVideo;Education;Utility;
Terminal=false
StartupWMClass=${BINARY_NAME}
EOF
    update_file "$DESKTOP_FILE2" "name, comment, exec, icon"
fi

# ===========================================================
# 5. Flatpak manifest
# ===========================================================
FLATPAK="$SCRIPT_DIR/com.vesta.desktop.flatpak.yml"
if [ -f "$FLATPAK" ]; then
    sed -i "s/^app-id: .*/app-id: ${APP_ID}/" "$FLATPAK"
    sed -i "s/^command: .*/command: ${BINARY_NAME}/" "$FLATPAK"
    update_file "$FLATPAK" "app-id, command"
fi

# ===========================================================
# 6. Workspace Cargo.toml (internal crate versions)
# ===========================================================
WORKSPACE_CARGO="$PROJECT_ROOT/Cargo.toml"
if [ -f "$WORKSPACE_CARGO" ]; then
    # Aggiorna solo le versioni delle internal crates (righe con path =)
    sed -i "/path = /s/version = \"[^\"]*\"/version = \"${VERSION}\"/" "$WORKSPACE_CARGO"
    update_file "$WORKSPACE_CARGO" "internal crate versions"
fi

# ===========================================================
# 7. Internal crate Cargo.toml package versions (core/lib/cli)
# ===========================================================
# cli/ è incluso qui perché check_internal_crate_versions.sh lo verifica: se
# uno dei due elenchi diverge dall'altro, il bump di release lascia le CLI
# indietro senza che nessun gate se ne accorga (è già successo).
while IFS= read -r crate_toml; do
    # Aggiorna la prima occorrenza di version nel file (sezione [package])
    sed -i "0,/^version = \".*\"/s//version = \"${VERSION}\"/" "$crate_toml"
    update_file "$crate_toml" "package version"
done < <(find "$PROJECT_ROOT/core" "$PROJECT_ROOT/lib" "$PROJECT_ROOT/cli" -mindepth 2 -maxdepth 2 -name Cargo.toml | sort)

# ===========================================================
# 9. Frontend Svelte Fallbacks (Sidebar.svelte, SettingsTab.svelte)
# ===========================================================
SIDEBAR_SVELTE="$PROJECT_ROOT/apps/srt-gui/src/lib/Sidebar.svelte"
if [ -f "$SIDEBAR_SVELTE" ]; then
    # Aggiorna fallback versione in Sidebar.svelte
    sed -i "s/appVersionNum = \"v[^\"]*\"/appVersionNum = \"v${VERSION}\"/g" "$SIDEBAR_SVELTE"
    sed -i "s/appVersionNum || \"v[^\"]*\"/appVersionNum || \"v${VERSION}\"/g" "$SIDEBAR_SVELTE"
    update_file "$SIDEBAR_SVELTE" "fallback version"
fi

SETTINGS_TAB_SVELTE="$PROJECT_ROOT/apps/srt-gui/src/lib/SettingsTab.svelte"
if [ -f "$SETTINGS_TAB_SVELTE" ]; then
    # Aggiorna fallback versione in SettingsTab.svelte
    sed -i "s/appVersionNum = \"v[^\"]*\"/appVersionNum = \"v${VERSION}\"/g" "$SETTINGS_TAB_SVELTE"
    update_file "$SETTINGS_TAB_SVELTE" "fallback version"
fi

# ===========================================================
# 8. Sincronizza Cargo.lock
# ===========================================================
if [ -f "$PROJECT_ROOT/Cargo.lock" ]; then
    echo -e "${YELLOW}Sincronizzo Cargo.lock (cargo update --workspace)...${NC}"
    (
        cd "$PROJECT_ROOT"
        cargo update --workspace
    )
fi

# ===========================================================
# 9. Verifica coerenza versioni interne
# ===========================================================
echo ""
echo -e "${YELLOW}🔎 Verifica coerenza versioni crate interni...${NC}"
bash "$SCRIPT_DIR/check_internal_crate_versions.sh"

echo ""
if [ $ERRORS -gt 0 ]; then
    echo -e "${RED}⚠ Completato con $ERRORS errori${NC}"
    exit 1
else
    echo -e "${GREEN}✅ Tutte le informazioni aggiornate alla versione ${VERSION}${NC}"
fi
