//! Ventarus: Android phone -> fullscreen mirror + audio + USB keyboard/mouse, fast.
//! Run it when you want the mirror: it starts one session and exits when you close it.
//! Sets `wm size` / `wm density` first, restores them afterwards.
//!
//! ventarus [--size WxH] [--density N] [--bitrate 16M] [--fps 60]
//!              [--fast] [--verbose] [--stats] [-- <extra scrcpy args>]

use std::{
    env, fs,
    path::PathBuf,
    process::{exit, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

struct Config {
    size: String,
    density: String,
    bitrate: String,
    fps: String,
    size_set: bool,
    bitrate_set: bool,
    fps_set: bool,
    verbose: bool,
    stats: bool,
    lite: bool,
    a4: bool,
    audio: bool,
    audio_codec: String,
    audio_buffer: Option<String>,
    density_set: bool,
    diagnose: bool,
    input: String,
    input_set: bool,
    extra: Vec<String>,
}

fn parse_args() -> Config {
    let mut c = Config {
        size: "1920x1080".into(),
        density: "200".into(),
        bitrate: "32M".into(),
        fps: "144".into(),
        size_set: false,
        bitrate_set: false,
        fps_set: false,
        verbose: false,
        stats: false,
        lite: false,
        a4: false,
        audio: true,
        audio_codec: "raw".into(),
        audio_buffer: None,
        density_set: false,
        diagnose: false,
        input: "aoa".into(),
        input_set: false,
        extra: vec![],
    };
    let mut it = env::args().skip(1);
    while let Some(a) = it.next() {
        let mut next = |name: &str| {
            it.next().unwrap_or_else(|| {
                eprintln!("missing value for {name}");
                exit(2)
            })
        };
        match a.as_str() {
            "--size" => {
                c.size = next("--size");
                c.size_set = true;
            }
            "--density" => {
                c.density = next("--density");
                c.density_set = true;
            }
            "--bitrate" => {
                c.bitrate = next("--bitrate");
                c.bitrate_set = true;
            }
            "--fps" => {
                c.fps = next("--fps");
                c.fps_set = true;
            }
            // Lighter preset: 720p, less to encode/decode/send.
            "--fast" => {
                c.size = "1280x720".into();
                c.density = "133".into();
                c.bitrate = "8M".into();
                c.bitrate_set = true;
                c.size_set = true;
            }
            "--verbose" | "-v" => c.verbose = true,
            "--stats" => c.stats = true,
            // Low-load preset for weak phones: 720p encode, 6M, 30 fps.
            "--lite" => c.lite = true,
            // Host-side preset for weak laptops (AMD A4-9120C Chromebooks and similar):
            // the CPU software-decodes the video, so send far less of it.
            "--a4" | "--chromebook" => c.a4 = true,
            "--no-audio" => c.audio = false,
            "--audio-codec" => {
                c.audio_codec = next("--audio-codec");
                if !["raw", "opus", "aac", "flac"].contains(&c.audio_codec.as_str()) {
                    eprintln!("--audio-codec must be raw, opus, aac or flac");
                    exit(2)
                }
            }
            // Audio buffer in ms; lower = less delay, but can crackle on slow machines.
            "--audio-buffer" => c.audio_buffer = Some(next("--audio-buffer")),
            "--diagnose" => c.diagnose = true,
            "--input" => {
                c.input = next("--input");
                c.input_set = true;
                if !["uhid", "aoa", "sdk"].contains(&c.input.as_str()) {
                    eprintln!("--input must be uhid, aoa or sdk");
                    exit(2)
                }
            }
            "--" => {
                c.extra = it.by_ref().collect();
                break;
            }
            "-h" | "--help" => {
                println!("ventarus [--size WxH] [--density N] [--bitrate 16M] [--fps 60] [--fast] [--lite] [--a4] [--no-audio] [--audio-codec raw|opus|aac|flac] [--audio-buffer MS] [--input aoa|uhid|sdk] [--diagnose] [--verbose] [--stats] [-- scrcpy args]");
                exit(0)
            }
            other => {
                eprintln!("unknown argument: {other}");
                exit(2)
            }
        }
    }
    if c.a4 {
        // 720p30 @ 4M, H.264 only. Explicit --size/--density/--bitrate/--fps still win.
        if !c.size_set {
            c.size = "1280x720".into();
        }
        if !c.density_set {
            c.density = "133".into();
        }
        if !c.bitrate_set {
            c.bitrate = "4M".into();
        }
        if !c.fps_set {
            c.fps = "30".into();
        }
    }
    c
}

fn adb(args: &[&str]) -> bool {
    Command::new("adb")
        .args(args)
        .stdout(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn adb_out(args: &[&str]) -> String {
    Command::new("adb")
        .args(args)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

fn reset_display(serial: &str) {
    // one adb call instead of two
    adb(&["-s", serial, "shell", "wm size reset; wm density reset"]);
}

/// Best hardware video encoder from `scrcpy --list-encoders` output: (codec, encoder).
/// Prefers h264, then h265, then av1.
fn pick_hw_encoder(listing: &str) -> Option<(String, String)> {
    for codec in ["h264", "h265", "av1"] {
        let tag = format!("--video-codec={codec} ");
        for line in listing.lines() {
            if !line.contains(&tag) || !line.contains("(hw)") {
                continue;
            }
            let key = "--video-encoder='";
            if let Some(start) = line.find(key) {
                let rest = &line[start + key.len()..];
                if let Some(end) = rest.find('\'') {
                    return Some((codec.to_string(), rest[..end].to_string()));
                }
            }
        }
    }
    None
}

fn cache_path(serial: &str) -> Option<PathBuf> {
    let base = env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))?;
    let safe: String = serial.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_').collect();
    Some(base.join("ventarus").join(format!("enc2-{safe}")))
}

/// Hardware (codec, encoder) for this phone. Looked up once (about a second), then cached.
fn hw_encoder(serial: &str, verbose: bool) -> Option<(String, String)> {
    let path = cache_path(serial)?;
    if let Ok(s) = fs::read_to_string(&path) {
        let s = s.trim().to_string();
        if s == "none" {
            return None; // known: this phone only has software encoders
        }
        if let Some((c, e)) = s.split_once(':') {
            return Some((c.to_string(), e.to_string()));
        }
        if !s.is_empty() {
            return Some(("h264".to_string(), s)); // older cache format
        }
    }
    if verbose {
        eprintln!("[ventarus] first run for this phone: looking up hardware encoder");
    }
    let out = Command::new("scrcpy").args(["-s", serial, "--list-encoders"]).output().ok()?;
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let enc = pick_hw_encoder(&text);
    if enc.is_none() && !text.contains("--video-encoder=") {
        return None; // listing failed; try again next time instead of caching
    }
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    let line = enc.as_ref().map(|(c, e)| format!("{c}:{e}")).unwrap_or_else(|| "none".to_string());
    let _ = fs::write(&path, line);
    enc
}


fn cmd_out(cmd: &str, args: &[&str]) -> String {
    Command::new(cmd)
        .args(args)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

fn env_or(k: &str) -> String {
    env::var(k).unwrap_or_else(|_| "-".into())
}

/// Names from `/proc/bus/input/devices` (`N: Name="..."` lines).
fn input_device_names(s: &str) -> Vec<String> {
    s.lines()
        .filter_map(|l| l.trim().strip_prefix("N: Name="))
        .map(|n| n.trim_matches('"').to_string())
        .collect()
}

/// One-shot report for debugging input problems: device info + a short scrcpy debug run.
fn diagnose(cfg: &Config) {
    println!("waiting for phone (plug in, USB debugging on)...");
    adb(&["wait-for-device"]);
    let serial = adb_out(&["get-serialno"]);
    let first = |s: String| s.lines().next().unwrap_or("").to_string();

    let mut report = String::new();
    report += &format!("scrcpy: {}\n", first(String::from_utf8_lossy(
        &Command::new("scrcpy").arg("--version").output().map(|o| o.stdout).unwrap_or_default()).to_string()));
    report += &format!("adb: {}\n", first(adb_out(&["version"])));
    report += &format!("input mode under test: {}\n", cfg.input);
    let dev = adb_out(&["shell",
        "echo android=$(getprop ro.build.version.release) sdk=$(getprop ro.build.version.sdk) model=$(getprop ro.product.model) flavor=$(getprop ro.build.flavor); \
         echo selinux=$(getenforce); ls -l /dev/uhid 2>&1; ls -l /dev/uinput 2>&1"]);
    report += &format!("--- phone ---\n{dev}\n");

    report += &format!(
        "--- host ---\nsession={} desktop={} wayland={} x11={}\n",
        env_or("XDG_SESSION_TYPE"), env_or("XDG_CURRENT_DESKTOP"), env_or("WAYLAND_DISPLAY"), env_or("DISPLAY")
    );
    let running = cmd_out("pgrep", &["-a", "scrcpy"]);
    report += &format!("scrcpy already running before this test:\n{}\n", if running.is_empty() { "(none)".to_string() } else { running });
    let listing = cmd_out("scrcpy", &["-s", &serial, "--list-encoders"]);
    let mut enc_lines: Vec<String> = Vec::new();
    for l in listing.lines().filter(|l| l.contains("encoder")) {
        enc_lines.push(l.trim().to_string());
    }
    report += &format!("--- encoders offered by the phone ---\n{}\n", enc_lines.join("\n"));
    let before = input_device_names(&adb_out(&["shell", "cat /proc/bus/input/devices"]));

    println!("running an 8 second scrcpy debug session: move the mouse and type in the phone window now...");
    let child = Command::new("scrcpy")
        .args(["-s", &serial, "-V", "debug"])
        .arg(format!("--keyboard={}", cfg.input))
        .arg(format!("--mouse={}", cfg.input))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    let mut during: Vec<String> = vec![];
    let mut top_out = String::new();
    let log = match child {
        Ok(mut ch) => {
            thread::sleep(Duration::from_secs(4));
            during = input_device_names(&adb_out(&["shell", "cat /proc/bus/input/devices"]));
            top_out = adb_out(&["shell", "top -b -n 1 -m 6"]);
            thread::sleep(Duration::from_secs(4));
            let _ = ch.kill();
            ch.wait_with_output()
                .map(|o| format!("{}\n{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr)))
                .unwrap_or_default()
        }
        Err(e) => format!("could not start scrcpy: {e}"),
    };

    let keys = ["uhid", "hid", "aoa", "input", "error", "warn", "fail", "denied", "permission", "exception", "usb"];
    let hits: Vec<&str> = log
        .lines()
        .filter(|l| {
            let l = l.to_lowercase();
            keys.iter().any(|k| l.contains(k))
        })
        .take(60)
        .collect();
    let new: Vec<&String> = during.iter().filter(|n| !before.contains(n)).collect();
    report += &format!(
        "--- input devices on the phone while mirroring ---\nnew since before scrcpy started: {}\nall: {}\n",
        if new.is_empty() { "(none)".to_string() } else { new.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ") },
        if during.is_empty() { "(could not read)".to_string() } else { during.join(", ") }
    );
    report += &format!("--- phone CPU while mirroring ---\n{top_out}\n");
    report += "--- scrcpy lines mentioning input/errors ---\n";
    report += &hits.join("\n");

    let path = env::temp_dir().join("ventarus-diagnose.txt");
    let _ = fs::write(&path, format!("{report}\n--- full log ---\n{log}"));
    println!("\n{report}\n\n(full log: {})", path.display());
}

fn main() {
    let cfg = parse_args();

    if cfg.diagnose {
        diagnose(&cfg);
        exit(0);
    }

    // One adb call finds the phone (and starts the adb server if needed).
    let serial = adb_out(&["get-serialno"]);
    let t0 = Instant::now();
    if serial.is_empty() || serial == "unknown" {
        eprintln!("ventarus: no phone found. Plug it in, enable USB debugging, accept the prompt, then run ventarus again.");
        eprintln!("          (if several phones are plugged in, unplug all but one)");
        exit(1);
    }

    {
        let serial = serial.clone();
        ctrlc::set_handler(move || {
            reset_display(&serial);
            exit(0);
        })
        .expect("failed to install signal handler");
    }

    // Start the display change and look up the encoder at the same time
    // (first run per phone: the lookup takes ~1s, so overlapping it matters).
    let wm = Command::new("adb")
        .args(["-s", &serial, "shell", &format!("wm size {}; wm density {}", cfg.size, cfg.density)])
        .stdout(Stdio::null())
        .spawn();
    let mut encoder = hw_encoder(&serial, cfg.verbose);
    if cfg.a4 && encoder.as_ref().is_some_and(|(c, _)| c != "h264") {
        encoder = None; // H.265/AV1 are much heavier for this CPU to decode
    }
    let t_enc = t0.elapsed();
    let wm_ok = match wm {
        Ok(mut w) => w.wait().map(|s| s.success()).unwrap_or(false), // must finish before scrcpy reads the display
        Err(_) => false,
    };
    if !wm_ok {
        eprintln!("ventarus: could not talk to the phone. Check it shows as \"device\" in `adb devices` (not \"unauthorized\").");
        exit(1);
    }
    let t_wm = t0.elapsed();

    // --lite: low-load preset for phones that can't keep up (e.g. software-only encoder).
    let (bitrate, fps) = if cfg.lite {
        (
            if cfg.bitrate_set { cfg.bitrate.clone() } else { "6M".to_string() },
            if cfg.fps_set { cfg.fps.clone() } else { "30".to_string() },
        )
    } else {
        (cfg.bitrate.clone(), cfg.fps.clone())
    };

    let build = |input: &str| -> Command {
        let mut cmd = Command::new("scrcpy");
        cmd.args(["-s", &serial])
            .args([
                "--fullscreen",
                "--window-borderless",
                "--video-buffer=0",
                "--video-codec-options=low-latency=1",
                "--no-mipmaps",
                "--turn-screen-off",
                "--stay-awake",
                "--no-clipboard-autosync",
            ])
            .arg(format!("--keyboard={input}"))
            .arg(format!("--mouse={input}"))
            .arg(format!("--video-bit-rate={bitrate}"))
            .arg(format!("--max-fps={fps}"));
        match &encoder {
            Some((codec, e)) => {
                cmd.arg(format!("--video-codec={codec}"));
                cmd.arg(format!("--video-encoder={e}"));
            }
            None => {
                cmd.arg("--video-codec=h264");
            }
        }
        if cfg.lite && !cfg.size_set {
            cmd.arg("--max-size=1280");
        }
        if cfg.audio {
            // default raw PCM: no encode/decode step (cheap over USB), starts and plays sooner
            cmd.arg(format!("--audio-codec={}", cfg.audio_codec));
            if let Some(ms) = &cfg.audio_buffer {
                cmd.arg(format!("--audio-buffer={ms}"));
            }
        } else {
            cmd.arg("--no-audio");
        }
        if cfg.stats {
            cmd.arg("--print-fps");
        }
        cmd.args(&cfg.extra);
        cmd
    };

    if cfg.verbose {
        eprintln!(
            "[ventarus] wm size/density: {:?}, encoder lookup done at {:?}, encoder: {}, bitrate {bitrate}, fps {fps}",
            t_wm,
            t_enc,
            encoder.as_ref().map(|(c, e)| format!("{c} {e}")).unwrap_or_else(|| "(software only)".to_string())
        );
    }

    if cfg.verbose {
        eprintln!(
            "[ventarus] wm size/density: {:?}, encoder lookup done at {:?}, encoder: {}, bitrate {bitrate}, fps {fps}",
            t_wm,
            t_enc,
            encoder.as_ref().map(|(c, e)| format!("{c} {e}")).unwrap_or_else(|| "(software only)".to_string())
        );
    }

    let started = Instant::now();
    let spawn_failed = |e: std::io::Error| -> ! {
        if e.kind() == std::io::ErrorKind::NotFound {
            eprintln!("ventarus: scrcpy not found in PATH (need 2.4+; 3.x recommended)");
        } else {
            eprintln!("ventarus: could not start scrcpy: {e}");
        }
        reset_display(&serial);
        exit(1)
    };
    let ok = match build(&cfg.input).status() {
        Ok(st) => st.success(),
        Err(e) => spawn_failed(e),
    };
    // AOA needs USB access to the phone; if it fails right at startup (and the
    // user didn't ask for it explicitly), fall back to UHID so the mirror still works.
    if !ok && cfg.input == "aoa" && !cfg.input_set && started.elapsed() < Duration::from_secs(5) {
        eprintln!("[ventarus] AOA input failed to start, falling back to UHID");
        let _ = build("uhid").status();
    }
    reset_display(&serial);
}

#[cfg(test)]
mod tests {
    use super::{input_device_names, pick_hw_encoder};

    #[test]
    fn parses_input_devices() {
        let s = "I: Bus=0003\nN: Name=\"scrcpy\"\nP: Phys=\nN: Name=\"gpio-keys\"\n";
        assert_eq!(input_device_names(s), vec!["scrcpy", "gpio-keys"]);
    }

    #[test]
    fn picks_first_hw_h264() {
        let s = "    --video-codec=h264 --video-encoder='c2.android.avc.encoder' (sw)\n\
    --video-codec=h264 --video-encoder='c2.qti.avc.encoder' (hw) [vendor]\n\
    --video-codec=h265 --video-encoder='c2.qti.hevc.encoder' (hw) [vendor]\n";
        assert_eq!(pick_hw_encoder(s), Some(("h264".into(), "c2.qti.avc.encoder".into())));
    }

    #[test]
    fn falls_back_to_hw_h265() {
        let s = "    --video-codec=h264 --video-encoder='c2.android.avc.encoder' (sw)\n\
    --video-codec=h265 --video-encoder='c2.intel.hevc.encoder' (hw)\n";
        assert_eq!(pick_hw_encoder(s), Some(("h265".into(), "c2.intel.hevc.encoder".into())));
    }

    #[test]
    fn none_when_only_software() {
        let s = "    --video-codec=h264 --video-encoder='c2.android.avc.encoder' (sw)\n";
        assert_eq!(pick_hw_encoder(s), None);
    }
}
