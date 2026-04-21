#!/bin/bash

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo -e "${BLUE}🚀 VESTA — Release Creator${NC}"
echo "=================================="

# ── Prepara e leggi versione dal PKGBUILD ─────────────────────
if [ ! -f "$SCRIPT_DIR/PKGBUILD" ]; then
    echo -e "${RED}❌ PKGBUILD non trovato in $SCRIPT_DIR${NC}"
    exit 1
fi

echo -e "${YELLOW}📝 Apertura di PKGBUILD con VS Code... (Salva e chiudi il file per continuare)${NC}"
code --wait "$SCRIPT_DIR/PKGBUILD"

VERSION=$(grep -Po '^pkgver=\K.*' "$SCRIPT_DIR/PKGBUILD")
if [ -z "$VERSION" ]; then
    echo -e "${RED}❌ Impossibile leggere pkgver dal PKGBUILD${NC}"
    exit 1
fi

TAG_VERSION="v$VERSION"
echo -e "${GREEN}✅ Versione rilevata: ${VERSION}${NC}"

# ── Aggiorna tutti i file del progetto ────────────────────────
echo -e "${YELLOW}🔄 Esecuzione update_project_info.sh...${NC}"
bash "$SCRIPT_DIR/update_project_info.sh"
echo ""

echo -e "${YELLOW}🔎 Verifica finale coerenza versioni interne...${NC}"
bash "$SCRIPT_DIR/check_internal_crate_versions.sh"
echo ""

# ── Verifica traduzioni i18n (blocca release su chiavi mancanti/vuote) ──
I18N_AUDIT_SCRIPT="$PROJECT_ROOT/apps/srt-gui/scripts/check_missing_translations.py"
I18N_REPORT="$PROJECT_ROOT/apps/srt-gui/reports/missing_translations_report.json"

if [ ! -f "$I18N_AUDIT_SCRIPT" ]; then
    echo -e "${RED}❌ Script audit traduzioni non trovato: $I18N_AUDIT_SCRIPT${NC}"
    exit 1
fi

PYTHON_BIN="python3"
if ! command -v "$PYTHON_BIN" &> /dev/null; then
    if command -v python &> /dev/null; then
        PYTHON_BIN="python"
    else
        echo -e "${RED}❌ Python non trovato (serve per l'audit traduzioni)${NC}"
        exit 1
    fi
fi

echo -e "${YELLOW}🌍 Verifica traduzioni (chiavi mancanti/vuote)...${NC}"
if ! "$PYTHON_BIN" "$I18N_AUDIT_SCRIPT" \
    --output "$I18N_REPORT" \
    --fail-on-issues \
    --block-reasons missing_key,empty_value; then
    echo -e "${RED}❌ Rilevate incongruenze nelle traduzioni.${NC}"
    echo -e "${YELLOW}   Correggi i file locale e riesegui la release.${NC}"
    echo -e "${YELLOW}   Report: $I18N_REPORT${NC}"
    exit 1
fi
echo -e "${GREEN}✅ Traduzioni OK (nessuna chiave mancante/vuota)${NC}"
echo ""

# ── Verifica e modifica release-notes ─────────────────────────
RELEASE_NOTES="$PROJECT_ROOT/docs/release-notes.md"
if [ ! -f "$RELEASE_NOTES" ]; then
    echo -e "${RED}❌ $RELEASE_NOTES non trovato${NC}"
    echo -e "   Crealo prima di fare una release."
    exit 1
fi

echo -e "${YELLOW}⚠  Le note di release verranno pubblicate con la GitHub Release.${NC}"
echo -e "${BLUE}📝 Apertura di release-notes.md con VS Code... (Salva e chiudi il file per continuare)${NC}"
code --wait "$RELEASE_NOTES"
echo ""
echo -e "${GREEN}✅ Release notes aggiornate e confermate${NC}"

# Conferma finale dopo aver visto/modificato le note
read -rp "Procedere con la release v${VERSION}? [s/N] " confirm_release
if [[ ! "$confirm_release" =~ ^[sS]$ ]]; then
    echo -e "${YELLOW}⚠  Release annullata.${NC}"
    exit 0
fi

# ── Verifica gh CLI ───────────────────────────────────────────
if ! command -v gh &> /dev/null; then
    echo -e "${RED}❌ GitHub CLI (gh) non installato${NC}"
    echo "   Installa con: sudo pacman -S github-cli"
    exit 1
fi

echo -e "${YELLOW}🔐 Verifica autenticazione GitHub...${NC}"
if ! gh auth status &> /dev/null; then
    echo -e "${RED}❌ Non sei autenticato con GitHub CLI${NC}"
    echo "   Esegui: gh auth login"
    exit 1
fi
echo -e "${GREEN}✅ Autenticato${NC}"

# ── Genera .SRCINFO ──────────────────────────────────────────
echo -e "${YELLOW}📄 Generazione .SRCINFO...${NC}"
cd "$SCRIPT_DIR"
if command -v makepkg &> /dev/null; then
    makepkg --printsrcinfo > .SRCINFO
    echo -e "${GREEN}✅ .SRCINFO aggiornato${NC}"
else
    echo -e "${YELLOW}⚠ makepkg non disponibile, skip .SRCINFO${NC}"
fi

# ── Sincronizza Cargo.lock con le nuove versioni ──────────────
echo -e "${YELLOW}🔒 Aggiornamento Cargo.lock (cargo update --workspace)...${NC}"
cd "$PROJECT_ROOT"
if ! cargo update --workspace 2>&1; then
    echo -e "${RED}❌ cargo update --workspace fallito${NC}"
    exit 1
fi
echo -e "${GREEN}✅ Cargo.lock sincronizzato${NC}"
echo ""

# ── Commit e tag ──────────────────────────────────────────────
echo -e "${YELLOW}📦 Commit e tag...${NC}"
cd "$PROJECT_ROOT"

# Aggiungi tutti i file modificati dallo script (incluso Cargo.lock)
git add -A

if git diff --cached --quiet; then
    echo -e "${YELLOW}⚠ Nessuna modifica da committare, procedo con il tag${NC}"
else
    git commit -m "chore: release v${VERSION}"
    echo -e "${GREEN}✅ Commit creato${NC}"
fi

# Se il tag esiste già, rimuovilo
if git rev-parse "$TAG_VERSION" >/dev/null 2>&1; then
    echo -e "${YELLOW}⚠ Tag $TAG_VERSION già esistente, lo ricreo${NC}"
    git tag -d "$TAG_VERSION"
    git push origin ":refs/tags/$TAG_VERSION" 2>/dev/null || true
fi

git tag "$TAG_VERSION"

# ── Push ──────────────────────────────────────────────────────
BRANCH=$(git branch --show-current)
echo -e "${YELLOW}🚀 Push su origin/${BRANCH} + tag ${TAG_VERSION}...${NC}"
git push origin "$BRANCH"
git push origin "$TAG_VERSION"
echo -e "${GREEN}✅ Push completato${NC}"

# ── Crea release GitHub ──────────────────────────────────────
echo -e "${YELLOW}🚀 Creazione release GitHub...${NC}"
gh release create "$TAG_VERSION" \
    --title "VESTA $TAG_VERSION" \
    --notes-file "$RELEASE_NOTES"

echo ""
echo -e "${GREEN}✅ Release $TAG_VERSION creata con successo!${NC}"
echo -e "${BLUE}   La GitHub Action 'Build and Release' partirà automaticamente.${NC}"
echo -e "${BLUE}   Dopo il build, esegui ./push-aur.sh per aggiornare AUR.${NC}"
