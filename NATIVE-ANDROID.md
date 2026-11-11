# X360 Nexus — Native Android Experimental (6.0)

Native Android APK: Java/Android SDK, OpenGL ES 2, Android AudioTrack and Rust JNI (ARM64-v8a).

## What actually works

- **CPU:** experimental big-endian PowerPC interpreter subset (integer, load/store, basic branches), bounded tests.
- **GPU:** software RGBA framebuffer in Rust + GPU-accelerated animated OpenGL ES 2.0 diagnostics (NOT Xbox Xenos).
- **Audio:** original PCM tone generated in Rust and played with Android AudioTrack. No Xbox XMA decoder.
- **Memory & kernel:** basic sandboxed guest memory, tiny test thread scheduler/ticks.
- **Controls:** touch virtual buttons, gamepad key events and Android joystick axes, saved in Rust controller state.
- **Library:** Android document picker; persisted URI/name/size; read-only XDVDFS ISO discovery, XEX2 header checks, without copying huge discs.
- **UI:** five native Android tabs, no WebView or HTML.

## Not yet implemented

Commercial Xbox 360 games, XEX execution, a complete Xenon triple-core PPC CPU, VMX128,
MMU/physical memory mapping, JIT, Xenos shaders and GPU command conversion,
Xbox kernel services, XMA decoding and compatibility for Forza/Minecraft.

## Build

GitHub Actions: `.github/workflows/x360-native-android.yml`, branch `feature/x360-nexus-native-0.4`.

`cargo test --manifest-path core/Cargo.toml`

GitHub workflow builds `libx360nexus.so` using NDK for arm64-v8a,
then `gradle :app:assembleDebug` and uploads a debug APK artifact.

Use only game files that you are authorized to use.
