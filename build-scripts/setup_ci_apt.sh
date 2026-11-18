#!/usr/bin/env bash
# Configure ephemeral GitHub Ubuntu runners before installing native dependencies.
set -euo pipefail

if [[ "${GITHUB_ACTIONS:-}" != "true" || "${RUNNER_OS:-}" != "Linux" ]]; then
  echo "This setup is restricted to Linux GitHub Actions runners." >&2
  exit 2
fi

apt_etc="${VESTA_APT_ETC:-/etc/apt}"
case "$(dpkg --print-architecture)" in
  amd64|i386) mirror="https://archive.ubuntu.com/ubuntu" ;;
  arm64|armhf|ppc64el|s390x) mirror="https://ports.ubuntu.com/ubuntu-ports" ;;
  *) echo "Unsupported Ubuntu runner architecture." >&2; exit 2 ;;
esac

sudo install -d "$apt_etc/apt.conf.d"
# Keep repository signatures and TLS verification enabled. Do not rewrite third-party sources.
if [[ -f "$apt_etc/apt-mirrors.txt" ]]; then
  printf '%s\n' "$mirror" | sudo tee "$apt_etc/apt-mirrors.txt" >/dev/null
fi
cat <<'CONFIG' | sudo tee "$apt_etc/apt.conf.d/80-vesta-ci" >/dev/null
Acquire::Retries "2";
Acquire::http::Timeout "30";
Acquire::https::Timeout "30";
Acquire::Languages "none";
APT::Install-Recommends "false";
APT::Update::Error-Mode "any";
DPkg::Lock::Timeout "120";
CONFIG

echo "Configured minimal Ubuntu CI dependencies with bounded network retries."
