#!/usr/bin/env bash
set -euo pipefail
command -v cargo >/dev/null || { echo "install Rust first: https://rustup.rs"; exit 1; }
command -v adb >/dev/null || echo "warning: adb not found (android-tools / platform-tools)"
command -v scrcpy >/dev/null || echo "warning: scrcpy not found (need 2.4+, 3.x recommended)"

# migrate from the old "phone-mirror" name if it is installed
systemctl --user disable --now phone-mirror.service 2>/dev/null || true
rm -f ~/.config/systemd/user/phone-mirror.service ~/.cargo/bin/phone-mirror
rm -rf "${XDG_CACHE_HOME:-$HOME/.cache}/phone-mirror"

cargo install --path . --force
mkdir -p ~/.config/systemd/user
cp linux/ventarus.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable ventarus.service
# restart (not just start): a service that is already running keeps the OLD binary otherwise
systemctl --user restart ventarus.service
# clear any stray scrcpy left from earlier manual runs
pkill -x scrcpy 2>/dev/null || true
echo "Done. Ventarus is installed and running. Plug in a phone with USB debugging enabled."
