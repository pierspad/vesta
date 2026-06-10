#!/usr/bin/env bash

set -euo pipefail
export LC_ALL=C

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
PKGBUILD="$SCRIPT_DIR/PKGBUILD"
WORKSPACE_CARGO="$PROJECT_ROOT/Cargo.toml"

if [ ! -f "$PKGBUILD" ]; then
    echo -e "${RED}❌ PKGBUILD non trovato in $SCRIPT_DIR${NC}"
    exit 1
fi

if [ ! -f "$WORKSPACE_CARGO" ]; then
    echo -e "${RED}❌ Cargo.toml workspace non trovato in $PROJECT_ROOT${NC}"
    exit 1
fi

# Estrae pkgver in modo portabile (compatibile anche con ambienti Bash su Windows).
VERSION=$(awk -F'=' '/^pkgver[[:space:]]*=/{print $2; exit}' "$PKGBUILD" | tr -d '\r' | sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//' -e 's/^"//' -e 's/"$//' || true)
if [ -z "$VERSION" ]; then
    echo -e "${RED}❌ Impossibile leggere pkgver da PKGBUILD${NC}"
    exit 1
fi

ERRORS=0

check_equals() {
    local label="$1"
    local file="$2"
    local current="$3"

    if [ -z "$current" ]; then
        echo -e "  ${RED}✗${NC} $label — valore non trovato"
        ((ERRORS++))
        return
    fi

    if [ "$current" != "$VERSION" ]; then
        echo -e "  ${RED}✗${NC} $label — trovato $current (atteso: $VERSION)"
        ((ERRORS++))
    else
        echo -e "  ${GREEN}✓${NC} $label — $current"
    fi
}

check_package_version() {
    local file="$1"
    local relative="${file#$PROJECT_ROOT/}"
    local current

    current=$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$file" | tr -d '\r' | head -n 1)
    if [ -z "$current" ]; then
        echo -e "  ${RED}✗${NC} $relative — versione package non trovata"
        ((ERRORS++))
        return
    fi

    if [ "$current" != "$VERSION" ]; then
        echo -e "  ${RED}✗${NC} $relative — version=$current (atteso: $VERSION)"
        ((ERRORS++))
    else
        echo -e "  ${GREEN}✓${NC} $relative — version=$current"
    fi
}

echo -e "${YELLOW}Controllo versioni package core/lib (atteso: $VERSION)...${NC}"
while IFS= read -r crate_toml; do
    check_package_version "$crate_toml"
done < <(find "$PROJECT_ROOT/core" "$PROJECT_ROOT/lib" -mindepth 2 -maxdepth 2 -name Cargo.toml | sort)

echo -e "${YELLOW}Controllo coerenza versioni app GUI (atteso: $VERSION)...${NC}"

# 1. package.json
PKG_JSON="$PROJECT_ROOT/apps/srt-gui/package.json"
if [ -f "$PKG_JSON" ]; then
    FRONTEND_VERSION=$(sed -n 's/.*"version": "\([^"]*\)".*/\1/p' "$PKG_JSON" | head -n 1 | tr -d '\r')
    check_equals "apps/srt-gui/package.json" "$PKG_JSON" "$FRONTEND_VERSION"
else
    echo -e "  ${RED}✗${NC} apps/srt-gui/package.json non trovato"
    ((ERRORS++))
fi

# 2. tauri.conf.json
TAURI_CONF="$PROJECT_ROOT/apps/srt-gui/src-tauri/tauri.conf.json"
if [ -f "$TAURI_CONF" ]; then
    TAURI_VERSION=$(sed -n 's/.*"version": "\([^"]*\)".*/\1/p' "$TAURI_CONF" | head -n 1 | tr -d '\r')
    check_equals "apps/srt-gui/src-tauri/tauri.conf.json" "$TAURI_CONF" "$TAURI_VERSION"
else
    echo -e "  ${RED}✗${NC} apps/srt-gui/src-tauri/tauri.conf.json non trovato"
    ((ERRORS++))
fi

# 3. src-tauri/Cargo.toml
TAURI_CARGO="$PROJECT_ROOT/apps/srt-gui/src-tauri/Cargo.toml"
if [ -f "$TAURI_CARGO" ]; then
    TAURI_CARGO_VERSION=$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$TAURI_CARGO" | head -n 1 | tr -d '\r')
    check_equals "apps/srt-gui/src-tauri/Cargo.toml" "$TAURI_CARGO" "$TAURI_CARGO_VERSION"
else
    echo -e "  ${RED}✗${NC} apps/srt-gui/src-tauri/Cargo.toml non trovato"
    ((ERRORS++))
fi

# 4. Cargo.lock
WORKSPACE_LOCK="$PROJECT_ROOT/Cargo.lock"
if [ -f "$WORKSPACE_LOCK" ]; then
    LOCK_VERSION=$(awk '
        /^\[\[package\]\]$/ { in_pkg=0; seen_pkg=1; next }
        seen_pkg && /^name = "vesta"$/ { in_pkg=1; next }
        in_pkg && /^version = / {
            value=$3
            gsub(/"/, "", value)
            print value
            exit
        }
    ' "$WORKSPACE_LOCK" | tr -d '\r')
    check_equals "Cargo.lock" "$WORKSPACE_LOCK" "$LOCK_VERSION"
else
    echo -e "  ${RED}✗${NC} Cargo.lock non trovato"
    ((ERRORS++))
fi

echo -e "${YELLOW}Controllo [workspace.dependencies] internal crates...${NC}"
for crate in srt-parser srt-extract srt-sync srt-translate; do
    line=$(grep -E "^${crate}\s*=\s*\{[^}]*path\s*=\s*\"" "$WORKSPACE_CARGO" || true)
    if [ -z "$line" ]; then
        echo -e "  ${RED}✗${NC} Cargo.toml — dipendenza $crate con path non trovata"
        ((ERRORS++))
        continue
    fi

    declared=$(echo "$line" | tr -d '\r' | sed -n 's/.*version = "\([^"]*\)".*/\1/p')
    if [ -z "$declared" ]; then
        echo -e "  ${RED}✗${NC} Cargo.toml — dipendenza $crate senza versione"
        ((ERRORS++))
        continue
    fi

    if [ "$declared" != "$VERSION" ]; then
        echo -e "  ${RED}✗${NC} Cargo.toml — $crate version=$declared (atteso: $VERSION)"
        ((ERRORS++))
    else
        echo -e "  ${GREEN}✓${NC} Cargo.toml — $crate version=$declared"
    fi
done

echo ""
if [ "$ERRORS" -gt 0 ]; then
    echo -e "${RED}❌ Coerenza versioni fallita (${ERRORS} errori)${NC}"
    exit 1
fi

echo -e "${GREEN}✅ Coerenza versioni crate interni verificata (${VERSION})${NC}"
