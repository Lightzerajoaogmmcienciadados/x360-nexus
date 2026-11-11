# X360 Nexus — Native Android Experimental (6.3)

Native Android APK: Java/Android SDK, OpenGL ES 2, Android AudioTrack and Rust JNI (ARM64-v8a).

## What actually works

- **CPU:** experimental big-endian PowerPC interpreter subset (integer, load/store, basic branches), bounded tests.
- **GPU:** software RGBA framebuffer in Rust + GPU-accelerated animated OpenGL ES 2.0 diagnostics (NOT Xbox Xenos).
- **Audio:** original PCM tone generated in Rust and played with Android AudioTrack. No Xbox XMA decoder.
- **Memory & kernel:** basic sandboxed guest memory, tiny test thread scheduler/ticks.
- **Controls:** touch virtual buttons, gamepad key events and Android joystick axes, saved in Rust controller state.
- **Library:** Android document picker; persisted URI/name/size; read-only XDVDFS ISO discovery, root `default.xex` file-offset and XEX2-header inspection, without copying huge discs.
- **Homebrew diagnostics:** opt-in execution of **raw big-endian PowerPC instructions** from `.ppc`, `.ppcbin`, `.nxpp` (64 KiB max, 20,000 instructions, simple sandbox; not XEX execution).
- **UI:** Android 15+ system bar insets, proper bottom navigation contrast and scroll reset on tab changes.
- **UI:** five native Android tabs, no WebView or HTML.

## Not yet implemented

Commercial Xbox 360 games, XEX execution, a complete Xenon triple-core PPC CPU, VMX128,
MMU/physical memory mapping, JIT, Xenos shaders and GPU command conversion,
Xbox kernel services, XMA decoding and compatibility for Forza/Minecraft.

## Build

GitHub Actions: `.github/workflows/x360-native-android-v63.yml`, branch `feature/x360-nexus-native-v6.3`.

`cargo test --manifest-path core/Cargo.toml`

GitHub workflow builds `libx360nexus.so` using NDK for arm64-v8a,
then `gradle :app:assembleDebug` and uploads a debug APK artifact.

Use only game files that you are authorized to use.

## 6.2 increments
- XEX2 header optionals with bounded big-endian parsing, including inside ISO.
- Experimental 4 KiB virtual memory mapping, R/W/X and overlap checks.
- Additional PPC ops; tests include execution from virtual address 0x82000000.
- Android native diagnostic improvements and a 100-800 Hz audio test slider.
- Still NO decrypt/mapping/boot of retail Xbox 360 games.

## Version 6.3: Offline cover gallery and haptics

- Native Android cover cards, miniatures and collapsible technical reports.
- One-tap assignment of local JPG, PNG or WebP cover to each imported game.
- Multi-selection cover import matches **normalized game filenames** or **Title ID**, which is parsed from a bounded XEX2 optional execution-info record (key 0x00040006).
- Persisted on-device internal JPEG thumbnail cache, including aliases for games imported later.
- Fallback Xbox-inspired placeholder if no user-provided cover exists.
- Optional haptic vibration for native buttons, gamepad touch presses and library actions (Android VIBRATE permission), toggle in Controls.
- Not an online cover browser: no official game art bundled, fetched or rehosted. Users supply their own artwork images.
- Cover support applies to any cataloged title, not a claim of automatic identification of every official release.
