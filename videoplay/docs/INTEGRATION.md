# Integrating dlos-videoplay into the DoglinkOS-2nd workspace

## Step 1 — Place the crate

```bash
cd /path/to/DoglinkOS-2nd
# Either clone directly or symlink:
ln -s /path/to/dlos-videoplay apps/videoplay
```

## Step 2 — Register in the workspace

Edit `Cargo.toml` at the repo root:

```toml
[workspace]
members = [
    "builder",
    "kernel",
    "app-rt",
    "apps/init",
    "apps/infinite-loop",
    "apps/imgview",
    "apps/ipc-demo",
    "apps/upppd",
    "apps/videoplay",   # ← add this line
]
resolver = "2"
default-members = ["builder"]
```

## Step 3 — Add as bindep in builder

Edit `builder/Cargo.toml`:

```toml
[dependencies.dlos-videoplay]
path = "../apps/videoplay"
artifact = "bin"
target = "x86_64-unknown-none"
default-features = false
features = ["dlos", "codec-mjpeg", "codec-xz"]
```

## Step 4 — Add to builder's initrd file list

Edit `builder/src/main.rs` `build_img()` function. Add `videoplay_path`
discovery (it's a bindep, but the env var name mangles hyphens):

```rust
let videoplay_env = env!("CARGO_BIN_FILE_VIDEOPLAY");
let videoplay_path = if videoplay_env.is_empty() {
    // Fallback: scan target dir for dlos-videoplay/<hash>/artifact/bin/videoplay
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.parent().unwrap();
    let mut found: Option<PathBuf> = None;
    for profile in &["release", "debug"] {
        let build_dir = workspace_root
            .join(format!("target/x86_64-unknown-none/{profile}/build"));
        if let Ok(entries) = std::fs::read_dir(&build_dir) {
            for entry in entries.flatten() {
                if !entry.file_name().to_string_lossy().starts_with("dlos-videoplay") {
                    continue;
                }
                if let Ok(sub_entries) = std::fs::read_dir(entry.path()) {
                    for sub in sub_entries.flatten() {
                        let candidate = sub.path().join("artifact/bin/videoplay");
                        if candidate.exists() {
                            found = Some(candidate);
                            break;
                        }
                    }
                }
                if found.is_some() { break; }
            }
        }
        if found.is_some() { break; }
    }
    found.unwrap_or_else(|| panic!("videoplay binary not found in target dir"))
} else {
    PathBuf::from(videoplay_env)
};
```

And add it + sample files to the initrd file map:

```rust
let initrd_files = BTreeMap::from([
    // ... existing entries ...
    ("/bin/videoplay", videoplay_path.to_path_buf()),
    ("/res/demo-mjpeg.avi", manifest_dir.parent().unwrap()
        .join("apps/videoplay/samples/demo-mjpeg.avi")),
    ("/res/demo.dlv", manifest_dir.parent().unwrap()
        .join("apps/videoplay/samples/demo.dlv")),
]);
```

## Step 5 — Add a build.rs to the builder

`builder/build.rs`:

```rust
fn main() {
    #[cfg(feature = "dlos")]
    { return; }
    #[cfg(not(feature = "dlos"))]
    {
        if let Ok(prefix) = std::env::var("SDL2_PREFIX") {
            let lib = format!("{prefix}/lib");
            let lib64 = format!("{prefix}/lib/x86_64-linux-gnu");
            println!("cargo:rustc-link-search=native={lib}");
            println!("cargo:rustc-link-search=native={lib64}");
        }
        println!("cargo:rustc-link-lib=dylib=SDL2");
    }
}
```

(This is for the sim target only; on dlos it's a no-op.)

## Step 6 — Enable serial output (optional)

Edit `builder/assets/limine.conf`:

```
timeout: 3
/DoglinkOS-2nd
    protocol: limine
    kernel_path: boot():/kernel
    cmdline: stdio=serial+tty
```

(The default `stdio=tty` only outputs to the framebuffer terminal;
`serial+tty` mirrors to the serial port so you can see kernel messages
via `-serial stdio` in QEMU.)

## Step 7 — Build

```bash
rustup toolchain install nightly
rustup +nightly component add rust-src
rustup target add x86_64-unknown-none
rustup +nightly target add x86_64-unknown-none

CARGO_UNSTABLE_BINDEPS=true cargo +nightly build -p builder --release
CARGO_UNSTABLE_BINDEPS=true ./target/release/builder
```

Result: `DoglinkOS-2nd.img` bootable UEFI image.

## Step 8 — Boot

```bash
qemu-system-x86_64 \
    -L /usr/share/qemu \
    -L /usr/share/seabios \
    -machine q35,accel=tcg \
    -m 512m -smp 1 -cpu qemu64,+x2apic \
    -drive if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/OVMF_CODE_4M.fd \
    -drive if=pflash,format=raw,file=/tmp/OVMF_VARS_4M.fd \
    -device ahci,id=ahci \
    -device ide-hd,drive=disk1,bus=ahci.0 \
    -drive if=none,format=raw,id=disk1,file=DoglinkOS-2nd.img \
    -display vnc=127.0.0.1:0 \
    -serial stdio
```

Once you see the shell prompt:

```
DoglinkOS Shell v1.4.1
[User@DoglinkOS-2nd /]$ videoplay
Video file path: /res/demo-mjpeg.avi
```

Connect a VNC viewer to `127.0.0.1:5900` to see the video.
