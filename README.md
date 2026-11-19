# Ventarus

**Plug in an Android phone and it's mirrored fullscreen on your computer, with sound, keyboard and mouse.**

Ventarus is a small Rust launcher for [scrcpy](https://github.com/Genymobile/scrcpy). It idles in the background, and the moment a phone is plugged in it resizes the phone's display, starts a tuned scrcpy session, and puts everything back when you unplug.

## Features

- **Fast start.** One adb call to wake up, setup steps run in parallel, and the phone's hardware encoder is looked up once and cached.
- **Real keyboard and mouse.** Uses scrcpy's AOA mode, so the phone sees a genuine USB keyboard and mouse over the same cable. No OTG adapter needed. Falls back to UHID if AOA can't start.
- **Clean.** Sets `wm size` and `wm density` on connect, and resets both when the mirror closes or the phone is unplugged.
- **Presets** for weak phones (`--lite`) and weak computers (`--a4`).
- **Set and forget.** Installs as a systemd user service that starts at login.
- **Tiny.** One source file, one dependency (`ctrlc`).

## Requirements

- [Rust](https://rustup.rs) (to build)
- `adb` (Android platform-tools)
- `scrcpy` 2.4 or newer (3.x recommended)
- USB debugging enabled and authorized on the phone

## Install (Linux)

```sh
git clone https://github.com/ilovesquishmallows3335/ventarus.git
cd ventarus
./install.sh
```

This builds Ventarus, installs it to `~/.cargo/bin`, and enables a systemd user service. The service idles on `adb wait-for-device` (near-zero CPU) and starts mirroring when you plug in a phone.

```sh
journalctl --user -u ventarus -f          # view logs
systemctl --user disable --now ventarus   # stop auto-start
```

Upgrading from the old `phone-mirror` name? `install.sh` removes the old service, binary and cache for you.

## Usage

```sh
ventarus [--size WxH] [--density N] [--bitrate 32M] [--fps 144]
         [--fast] [--lite] [--a4] [--input aoa|uhid|sdk]
         [--diagnose] [--verbose] [--stats] [-- <extra scrcpy args>]
```

| Option | Meaning |
| --- | --- |
| `--size WxH` | Phone resolution while mirroring (default `1920x1080`) |
| `--density N` | Phone DPI while mirroring (default `200`) |
| `--bitrate` | Video bitrate (default `32M`) |
| `--fps` | Frame rate cap (default `144`; the phone's display and encoder set the real limit) |
| `--fast` | Lighter preset: 1280x720, 8M |
| `--lite` | Low-load preset for weak **phones**: 720p, 6M, 30 fps |
| `--a4` / `--chromebook` | Low-load preset for weak **computers** (see below): 720p, 4M, 30 fps, H.264 only |
| `--input` | How keyboard and mouse reach the phone: `aoa` (default), `uhid` or `sdk` |
| `--diagnose` | Prints phone and scrcpy info plus input-related log lines from an 8 second debug run (saved as `ventarus-diagnose.txt` in the system temp dir) |
| `--verbose` | Prints startup timings |
| `--stats` | Prints FPS |
| `-- ...` | Everything after `--` goes straight to scrcpy and overrides Ventarus's defaults |

Explicit `--size`, `--density`, `--bitrate` and `--fps` always beat a preset.

```sh
ventarus --size 2560x1440 --density 280 --bitrate 48M --fps 120
ventarus -- --video-codec=h265
```

Release the grabbed mouse and keyboard with left Alt (scrcpy's MOD key). Quit with MOD+Q or by closing the window.

## Older or low-power computers

scrcpy decodes video in software on the computer, so the defaults (1080p, 32 Mbps, up to 144 fps) are too much for chips like the AMD A4-9120C. Start with the preset:

```sh
ventarus --a4
```

It sends 720p at 4 Mbps and 30 fps, and sticks to H.264 because H.265 and AV1 are much harder to decode. From there:

| Goal | Try |
| --- | --- |
| Smoother motion | `ventarus --a4 --fps 60` |
| Still choppy | `ventarus --a4 --size 960x540 --density 100 --bitrate 3M` |
| Make it permanent | Add `--a4` to `ExecStart` in `~/.config/systemd/user/ventarus.service`, then `systemctl --user daemon-reload && systemctl --user restart ventarus` |

Higher frame rates need more than a flag: the decoder is the limit, not Ventarus. Most laptop panels are 60 Hz anyway, so frames above 60 fps are never shown.

## How it stays fast

- **Hardware encoder, picked once.** The best hardware encoder (H.264, then H.265, then AV1) is looked up on first plug-in and cached in `~/.cache/ventarus/`.
- **One adb call to wake up.** Waiting for the phone and reading its serial share a single adb process.
- **Overlapped setup.** The display resize and the encoder lookup run at the same time.
- **Raw audio.** PCM over USB skips the encode and decode steps. Switch back with `-- --audio-codec=opus`.
- **Low-latency video.** Zero video buffer plus scrcpy's low-latency codec option.
- **No polling on unplug.** Ventarus blocks on `adb wait-for-disconnect`, polling only on old adb versions.

Most of the remaining delay is scrcpy's own start-up on the phone, which Ventarus can't skip.

## Troubleshooting

- **Nothing happens on plug-in.** Check `adb devices` shows the phone as `device` (not `unauthorized`), and look at `journalctl --user -u ventarus -f`.
- **Keyboard or mouse don't work.** Run `ventarus --diagnose`, or try `--input uhid`.
- **Phone display stuck at the wrong size.** Run `adb shell "wm size reset; wm density reset"`.

## Uninstall

```sh
./uninstall.sh
```

Removes the service, binary, cache and any stray scrcpy, and resets the phone's display override if it's plugged in. `adb` and `scrcpy` stay installed.

## Windows and macOS

Ventarus builds the same way (`cargo build --release`). Run the executable from a shortcut or startup folder with `adb` and `scrcpy` on your PATH. The systemd service and install scripts are Linux only.

## Development

```sh
cargo test
cargo build --release
```

## License

Add a license before publishing (for example MIT or Apache-2.0), then name it here.
