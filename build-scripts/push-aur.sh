#!/usr/bin/env bash
# ===========================================================================
# push-aur.sh
# Aggiorna il repository AUR con il PKGBUILD corrente.
# Da eseguire DOPO che la GitHub Release è stata creata e i .deb sono online.
# ===========================================================================

set -euo pipefail
export LC_ALL=C

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PKGBUILD="$SCRIPT_DIR/PKGBUILD"
UPDATE_SCRIPT="$SCRIPT_DIR/update_project_info.sh"
CHECK_SCRIPT="$SCRIPT_DIR/check_internal_crate_versions.sh"

PROJECT_NAME="${PROJECT_NAME:-$(awk -F'=' '/^pkgname[[:space:]]*=/{print $2; exit}' "$PKGBUILD" | tr -d '\r' | sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//')}"
AUR_REPO_DIR="${AUR_REPO_DIR:-$SCRIPT_DIR/${PROJECT_NAME}}"
AUR_REMOTE_URL="ssh://aur@aur.archlinux.org/${PROJECT_NAME}.git"

if [ -z "$PROJECT_NAME" ]; then
    echo -e "${RED}❌ Impossibile leggere pkgname dal PKGBUILD${NC}"
    exit 1
fi

if [ ! -f "$UPDATE_SCRIPT" ] || [ ! -f "$CHECK_SCRIPT" ]; then
    echo -e "${RED}❌ Script update/check mancanti in $SCRIPT_DIR${NC}"
    exit 1
fi

echo -e "${BLUE}🔄 AUR Push — ${PROJECT_NAME}${NC}"
echo "=================================="

echo -e "${YELLOW}🔄 Allineamento metadati dal PKGBUILD...${NC}"
bash "$UPDATE_SCRIPT"

echo -e "${YELLOW}🔎 Verifica coerenza versioni interne...${NC}"
bash "$CHECK_SCRIPT"

cd "$SCRIPT_DIR"

# ── Clona il repo AUR se non esiste ──────────────────────────
if [[ -d "$AUR_REPO_DIR/.git" ]] && git -C "$AUR_REPO_DIR" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    CURRENT_REMOTE_URL="$(git -C "$AUR_REPO_DIR" remote get-url origin 2>/dev/null || true)"
    if [[ "$CURRENT_REMOTE_URL" != "$AUR_REMOTE_URL" ]]; then
        echo -e "${RED}❌ $AUR_REPO_DIR punta a un remote diverso da AUR.${NC}"
        echo -e "${RED}   Remote attuale: ${CURRENT_REMOTE_URL:-<nessuno>}${NC}"
        echo -e "${RED}   Remote atteso:  $AUR_REMOTE_URL${NC}"
        exit 1
    fi
else
    if [[ -e "$AUR_REPO_DIR" ]]; then
        BACKUP_DIR="${AUR_REPO_DIR}.backup.$(date +%s)"
        echo -e "${YELLOW}⚠ $AUR_REPO_DIR esiste ma non e' un repository git AUR valido.${NC}"
        echo -e "${YELLOW}📦 Backup in $BACKUP_DIR${NC}"
        mv "$AUR_REPO_DIR" "$BACKUP_DIR"
    else
        echo -e "${YELLOW}⚠ Directory $AUR_REPO_DIR non trovata.${NC}"
    fi

    echo -e "${YELLOW}⬇️  Clonazione repo AUR...${NC}"
    if ! git clone "$AUR_REMOTE_URL" "$AUR_REPO_DIR"; then
        echo -e "${RED}❌ Errore nella clonazione. Configura la chiave SSH per AUR.${NC}"
        exit 1
    fi
fi

# ── Pulisci artefatti precedenti ─────────────────────────────
rm -rf pkg/ src/ ./*.pkg.*

# ── Aggiorna checksum SHA256 ─────────────────────────────────
echo -e "${YELLOW}🔍 Aggiornamento checksum con updpkgsums...${NC}"
if ! command -v updpkgsums >/dev/null 2>&1; then
    echo -e "${RED}❌ updpkgsums non trovato. Installa pacman-contrib.${NC}"
    exit 1
fi
updpkgsums
echo -e "${GREEN}✅ Checksum aggiornati${NC}"

# ── Genera .SRCINFO ──────────────────────────────────────────
echo -e "${YELLOW}📄 Generazione .SRCINFO...${NC}"
makepkg --printsrcinfo > .SRCINFO
echo -e "${GREEN}✅ .SRCINFO generato${NC}"

# ── Copia file nel repo AUR ──────────────────────────────────
echo -e "${YELLOW}📁 Copia file nel repository AUR...${NC}"
cp PKGBUILD .SRCINFO "$AUR_REPO_DIR/"

# ── Commit e push ─────────────────────────────────────────────
cd "$AUR_REPO_DIR"
git add -A

if git diff --staged --quiet; then
    echo -e "${YELLOW}⚠ Nessuna modifica da pushare su AUR${NC}"
    exit 0
fi

VERSION=$(awk -F'=' '/^pkgver[[:space:]]*=/{print $2; exit}' PKGBUILD | tr -d '\r' | sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//')

echo -e "${YELLOW}Modifiche che verranno pushate su AUR (v${VERSION}):${NC}"
git diff --staged --stat
echo ""

read -rp "Procedere con il push su AUR di v${VERSION}? [s/N] " confirm_aur
if [[ ! "$confirm_aur" =~ ^[sS]$ ]]; then
    echo -e "${YELLOW}Push AUR annullato.${NC}"
    exit 0
fi

echo -e "${YELLOW}🚀 Commit e push su AUR...${NC}"
git commit -m "Update to v${VERSION}"
git push
echo -e "${GREEN}✅ Push completato su AUR${NC}"
