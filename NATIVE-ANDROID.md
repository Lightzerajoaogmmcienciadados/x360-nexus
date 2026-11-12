# X360 Nexus — Xenia Mali Integrado (10.1)

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

GitHub Actions: `.github/workflows/x360-native-android-v66.yml`, branch `feature/x360-nexus-native-v6.6`.

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


## Version 6.4: GPU Xenos Lab and vibration debugging

- **Xenos Lab is an independent software diagnostic**, NOT Xbox 360 Xenos graphics:
  - original bounded GPU command queue, 320×180 ARGB framebuffer
  - clear, scissor rectangles, software rasterization of filled triangles
  - source-over alpha blending, procedural checkerboard texture sampling
  - three test scenes, JNI preview, counts of commands/pixels and clipping
  - automated Rust tests for drawing, scissor, blending, limits and scene generation
- Android diagnostic panel renders the Rust framebuffer as a native Bitmap.
- Device haptic test reports hardware, Android haptic setting and requested strength;
  explicit two-pulse motor test, fallback native View feedback, duration increased.
- Library adds search, favorites and alphabetical/recent ordering on-device.
- Not implemented: actual Xenos PM4 packet decoding, Xenos registers,
  shader microcode, EDRAM or commercial Xbox 360 game GPU emulation.
- Device haptic test must be verified on the phone: API success alone cannot
  prove physical vibration. Galaxy/Android settings may disable it.


## Version 6.5: experimental 3D GPU pipeline
- New software rasterizer in core/src/xenos_pipeline.rs alongside existing v6.4 GPU LAB.
- Z-buffer (depth compare), indexed triangle meshes, per-vertex color interpolation
  (Gouraud shading), bilinear sampling of procedural 8x8 textures, scissor rectangle.
- A perspective-projected textured cube drawn from 12 triangles (two per cube face).
- New Android scenes: Z-buffer, Gouraud, Cubo 3D, in addition
  to the original Pista, Triângulos, Texturas.
- 320x180 CPU software render, copied through Rust JNI into Android Bitmap;
  counts draw calls, indexed triangles, shaded fragments, Z-test rejects,
  and bilinear texture samples.
- Rust unit tests for depth order, color interpolation, filtering,
  index and NaN validation, 3D scene generation.
- Working v6.4 Android haptic test and vibration preferences retained.
- Not an Xbox 360 Xenos emulator: no actual Xenos command decoding,
  shader translation, EDRAM tiling, texture formats or commercial game execution.

## Version 6.6 — broader experimental diagnostics

- Additional **PowerPC** opcodes: rlwinm, lha, lbzu, lwzu, stwu, slw/srw, cmplw and bctr.
- A **cooperative** diagnostic kernel thread scheduler (Ready/Waiting/Stopped).
- Stereo **PCM mixing with saturation** and basic linear resampling.
- Read-only recognition of **STFS CON, LIVE and PIRS** headers; not full container mounting.
- The native app now reports unknown file signatures using the new Rust inspector.
- A new **Compat Lab** panel runs combined native CPU, kernel, audio and GPU tests.
- Existing Z-buffer/Gouraud/cube GPU lab, game cover support and Android vibration preserved.

Commercial Xbox 360 game execution remains unsupported. It still lacks an Xbox 360
XEX image loader, decryption, full Xenon PPC/VMX128/MMU, XAM/Xbox kernel APIs,
Xenos register/packet/shader emulation, and XMA audio decoding.


## v10.0 (experimental)
- Native GPU stress test: 30 frames / 24 textured triangles per frame, depth test, software rasterization, timings and draw statistics.
- Benchmark runs off the Android UI thread to avoid blocking input and rendering.
- Keeps the Android haptic vibration fix, game cover library, ISO/XEX metadata, CPU subset, PCM and gamepad support.
- NOT compatible with commercial Xbox 360 games: real Xenon JIT/MMU, Xbox kernel APIs, Xenos shader/packet translation and XMA are still missing.


## v10.0 - Real-game launch through installed XenDroid Mali

The X360 Nexus v10.0 APK remains a native Android/Rust companion.
**It does not bundle or embed Xenia's C++ emulator**. Instead, it integrates
an explicitly installed Android Xbox 360 emulator from the independent XenDroid
Mali/Xenia family. The launch bridge is in ExternalXboxBackend.java.

Upstream XenDroid Mali: https://github.com/DEGROUCHO/xendroid-mali
Source of the exported Android activity and game URI contract:
https://github.com/DEGROUCHO/xendroid-mali/blob/main/src/xendroid-src/app/src/main/AndroidManifest.xml
https://github.com/DEGROUCHO/xendroid-mali/blob/main/src/xendroid-src/app/src/main/java/xendroid/compose/core/FrontendLaunch.kt
License: https://github.com/DEGROUCHO/xendroid-mali/blob/main/LICENSE

Installation: Install XenDroid Mali separately, then open X360 Nexus Library,
select your own legally obtained ISO/XEX, and tap 'Jogar via XenDroid Mali'.
The app dispatches xendroid.intent.action.xendroid to package xendroid.compose,
exported EmulatorHostActivity, with game_uri and an Android SAF read-only grant.
XenDroid, not X360 Nexus Rust, performs ARM64 JIT, emulated Xbox kernel, XMA and
Vulkan-based GPU work.

Limits: The upstream fork mainly reports results on a Mali-G720 device.
Lower-end Mali phones such as Galaxy A17 are not validated. Vulkan availability,
Android provider permissions, ARM64 memory, shaders and performance may prevent
games from booting. No retail game compatibility is guaranteed, and this app's
internal Xenon CPU/Xenos GPU are still limited diagnostic implementations.
No game files, Xbox system keys or retail games are included.


## 10.1 - Integrated Xenia runtime (single APK; experimental)

- Repackages the official XenDroid Ultimate Mali q664 APK under BSD 3-Clause with the original Kotlin UI and Xenia ARM64/Vulkan backend.
- Original XenDroid Application, emulation process, libe.so, Vulkan runtime and assets remain in the APK.
- X360 Nexus Java dex and native libx360nexus.so are added, and Nexus is the launcher.
- Game action launches the XenDroid in-package emulation activity with the selected SAF URI.
- Original BSD license preserved in assets/X360_NEXUS_XENIA_ENGINE_NOTICES.txt; source at github.com/DEGROUCHO/xendroid-mali.
- Built using tools/build_xenia_integrated.py and GitHub Actions.
- Signed custom package uses xendroid.compose; installing alongside release-signed XenDroid can fail with signing mismatch. Uninstalling XenDroid may delete app data.
- This is binary integration with an open-source Xenia fork, NOT a newly implemented full Xenia core in Rust.
- Compatibility and performance on Galaxy A17 have not been independently tested. No games, BIOS or keys bundled.
