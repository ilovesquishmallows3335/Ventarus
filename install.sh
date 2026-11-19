#!/usr/bin/env bash
set -euo pipefail
command -v cargo >/dev/null || { echo "install Rust first: https://rustup.rs"; exit 1; }
command -v adb >/dev/null || echo "warning: adb not found (android-tools / platform-tools)"
command -v scrcpy >/dev/null || echo "warning: scrcpy not found (need 2.4+, 3.x recommended)"

# Older versions auto-started on plug-in via a systemd service. Remove it (and the pre-rename one).
for svc in ventarus phone-mirror; do
    systemctl --user disable --now "$svc.service" 2>/dev/null || true
    rm -f "$HOME/.config/systemd/user/$svc.service"
done
systemctl --user daemon-reload 2>/dev/null || true
rm -f ~/.cargo/bin/phone-mirror
rm -rf "${XDG_CACHE_HOME:-$HOME/.cache}/phone-mirror"

cargo install --path . --force
echo "Done. Plug in your phone (USB debugging on) and run: ventarus"
