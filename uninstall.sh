#!/usr/bin/env bash
# Removes ventarus: service, binary, cache, stray scrcpy, and resets the phone's display override.
set -u

systemctl --user disable --now ventarus.service phone-mirror.service 2>/dev/null || true
rm -f "$HOME/.config/systemd/user/phone-mirror.service" "$HOME/.cargo/bin/phone-mirror"
rm -f "$HOME/.config/systemd/user/ventarus.service"
systemctl --user daemon-reload 2>/dev/null || true

cargo uninstall ventarus 2>/dev/null || true
rm -f "$HOME/.cargo/bin/ventarus"

pkill -x scrcpy 2>/dev/null || true
rm -rf "${XDG_CACHE_HOME:-$HOME/.cache}/ventarus" /tmp/ventarus-diagnose.txt

# put the phone's resolution/density back if it is plugged in
if command -v adb >/dev/null 2>&1; then
    timeout 5 adb shell "wm size reset; wm density reset" >/dev/null 2>&1 || true
fi

if [ -f /etc/udev/rules.d/99-phone-mirror.rules ]; then
    echo "Old udev rule found; remove it with:"
    echo "  sudo rm /etc/udev/rules.d/99-phone-mirror.rules && sudo udevadm control --reload"
fi

echo "ventarus uninstalled. (adb and scrcpy were left installed.)"
echo "You can now delete this folder."
