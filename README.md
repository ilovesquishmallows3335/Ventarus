# Ventarus

**Mirror your Android phone fullscreen on your computer, with sound, keyboard and mouse. One command, fast start.**

Ventarus is a small Rust launcher for [scrcpy](https://github.com/Genymobile/scrcpy). Plug in your phone, run `ventarus`, and it resizes the phone's display, starts a tuned scrcpy session, and puts everything back when you close the window.

Ventarus does **not** start by itself when you plug a phone in. It runs only when you launch it.

## Features

- **Fast start.** About two adb calls before scrcpy launches, setup steps run in parallel, and the phone's hardware encoder is looked up once and cached.
- **Real keyboard and mouse.** Uses scrcpy's AOA mode, so the phone sees a genuine USB keyboard and mouse over the same cable. No OTG adapter needed. Falls back to UHID if AOA can't start.
- **Audio** on by default, with `--no-audio`, `--audio-codec` and `--audio-buffer` to tune or turn it off.
- **Clean.** Sets `wm size` and `wm density` on start, and resets both when you close the mirror or press Ctrl+C.
- **Presets** for weak phones (`--lite`) and weak computers (`--a4`).
- **Tiny.** One source file, one dependency (`ctrlc`).

## Requirements

- [Rust](https://rustup.rs) (to build)
- `adb` (Android platform-tools)
- `scrcpy` 2.4 or newer (3.x recommended)
- USB debugging enabled and authorized on the phone

## Install

```sh
git clone https://github.com/ilovesquishmallows3335/ventarus.git
cd ventarus
./install.sh
```

This builds Ventarus and installs it to `~/.cargo/bin` (make sure that's on your `PATH`). If you used an older version that started on plug-in, `install.sh` also removes its auto-start service.

## Usage

Plug in your phone, then run:

```sh
ventarus
```

If no phone is found, Ventarus tells you and exits instead of waiting.

```sh
ventarus [--size WxH] [--density N] [--bitrate 32M] [--fps 144]
         [--fast] [--lite] [--a4] [--input aoa|uhid|sdk]
         [--no-audio] [--audio-codec raw|opus|aac|flac] [--audio-buffer MS]
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
| `--no-audio` | Mirror video only |
| `--audio-codec` | `raw` (default, lowest latency), `opus`, `aac` or `flac` |
| `--audio-buffer MS` | Audio buffer in milliseconds. Lower means less delay but can crackle on slow machines |
| `--diagnose` | Waits for a phone, then prints phone and scrcpy info plus input-related log lines from an 8 second debug run (saved as `ventarus-diagnose.txt` in the system temp dir) |
| `--verbose` | Prints startup timings |
| `--stats` | Prints FPS |
| `-- ...` | Everything after `--` goes straight to scrcpy and overrides Ventarus's defaults |

Explicit `--size`, `--density`, `--bitrate` and `--fps` always beat a preset.

```sh
ventarus --size 2560x1440 --density 280 --bitrate 48M --fps 120
ventarus -- --video-codec=h265
```

Release the grabbed mouse and keyboard with left Alt (scrcpy's MOD key). Quit with MOD+Q or by closing the window.

### Launch it with one key

Since Ventarus no longer auto-starts, bind it to a shortcut or add an alias:

```sh
alias phone='ventarus'    # in ~/.bashrc or ~/.zshrc
```

Or create a desktop launcher or a keyboard shortcut in your desktop environment that runs `ventarus`.

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

Higher frame rates need more than a flag: the decoder is the limit, not Ventarus. Most laptop panels are 60 Hz anyway, so frames above 60 fps are never shown.

## How it stays fast

- **No waiting, no extra checks.** Ventarus finds the phone with one adb call and doesn't pre-check scrcpy; it just launches it.
- **Hardware encoder, picked once.** The best hardware encoder (H.264, then H.265, then AV1) is looked up on first run and cached in `~/.cache/ventarus/`.
- **Overlapped setup.** The display resize and the encoder lookup run at the same time.
- **Raw audio.** PCM over USB skips the encode and decode steps. Switch with `--audio-codec opus`.
- **Low-latency video.** Zero video buffer plus scrcpy's low-latency codec option.

Most of the remaining delay is scrcpy's own start-up on the phone, which Ventarus can't skip.

## Troubleshooting

- **"no phone found".** Check `adb devices` lists the phone as `device` (not `unauthorized` or empty).
- **Keyboard or mouse don't work.** Run `ventarus --diagnose`, or try `--input uhid`.
- **Phone display stuck at the wrong size.** Run `adb shell "wm size reset; wm density reset"`.

## Uninstall

```sh
./uninstall.sh
```

Removes the binary, cache and any old auto-start service, and resets the phone's display override if it's plugged in. `adb` and `scrcpy` stay installed.

## Windows and macOS

Ventarus builds the same way (`cargo build --release`). Run the executable from a shortcut with `adb` and `scrcpy` on your PATH. `install.sh` and `uninstall.sh` are for Linux.

## Development

```sh
cargo test
cargo build --release
```

## License

Add a license before publishing (for example MIT or Apache-2.0), then name it here.
