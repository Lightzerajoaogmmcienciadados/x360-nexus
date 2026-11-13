# X360 GALAXY v5.2 — PM4 guest-memory bridge, partial HLE, protected key input

Incremental update of the working v5.1 XEX2/ISO/AES/LZX/CPU stack. The existing source is preserved.

## New in v5.2

- **Private hexadecimal XEX key entry**: Android dialog validates precisely 32 hex digits, accepts a user-provided 16-byte key, never prints the key or embeds keys in the APK. The optional `keys.bin` reader still works.
- **Encrypted persistence**: AndroidKeyStore generates an AES-256-GCM wrapping key. SharedPreferences stores **only authenticated ciphertext and IV**, never a plaintext key or its hex form. Keys can become unreadable after OS reset/Keystore changes and must be re-entered. `allowBackup=false` remains enabled.
- **PM4**: safe Type 0/2/3 parser now recognizes `LOAD_ALU_CONSTANT (0x2F)` with bounded, injected guest-memory reader, Type 3 NOP, and shader register constants. No guest shader translation is claimed; unsupported DRAW_INDX is logged. Tests include valid/invalid memory reads.
- **Rust RAM -> PM4 C++**: `nativeReadSessionMemory` provides bounded 16-KiB guest-memory reads; `nativeSubmitPm4Bytes` decodes big-endian DWORDs from the current CPU session. There is no unrestricted guest memory pointer or 512-MiB copy.
- **Guest command ring for original test software only**: private HLE service 20 maps 64 KiB at `0x9F000000`; service 21 sets the number of submitted DWORDs (up to 4096); existing service 15 VdSwap increments its counter. Kotlin observes the new `GPU_RING`, `GPU_DWORDS`, and `SWAPS` diagnostics and submits readable packets to C++ when the swap count advances.
- **Additional *private test ABI* services**: service 22 reports an offline local test profile (no Xbox authentication); 23 yields to host thread. These are not Xbox 360 ordinals, and real `xboxkrnl.exe` / `xam.xex` HLE is still missing.
- **Android**: v5.2 name/version and protection against plaintext key storage; same EGL / GLES diagnostic video surface and controller overlay.

## Current limitations — important

This is **not** a compatible Xbox 360 game emulator. Commercial ISO boot is not proven and Minecraft/Forza will not run correctly. A complete Xenos shader translator, Xenos register/MMIO/DMA interception, EDRAM resolves, real shader fetching, textures and fully functional `xboxkrnl/xam` ordinal IAT are **not implemented**. The GLES triangle is an original diagnostic shader; PM4 constants affect only test visualization. Vulkan remains a setting request, not an active backend. We do **not** claim it is a real game frame.

A game cannot be made to work by returning zero for unknown imports: unsupported calls must be visible and execution paused. No built-in retail/devkit keys, games or firmware are distributed.

## Build

`cmake -S . -B build/host && cmake --build build/host && ctest --test-dir build/host --output-on-failure`

Android ARM64 APK is built by `.github/workflows/build-x360-android-apk.yml` (Java 17, Rust stable, Android SDK 35, NDK 27.2.12479018, CMake 3.22.1, Gradle 8.10.2).

## Updated files

- `android/app/src/main/java/com/x360nexus/SecretKeyStore.kt` (new)
- `android/app/src/main/java/com/x360nexus/MainActivity.kt`
- `android/app/src/main/cpp/xenos_pm4.{hpp,cpp}`, `gpu_pm4_jni.cpp`, `xenos_video.cpp`
- `core/src/kernel.rs`, `core/src/runtime.rs`
- `tests/pm4_tests.cpp`
- Android manifest, layout, Gradle, CMakeLists, Cargo.toml, workflow, README

Reference for actual PM4 opcode values and register classes: [Xenia GPU/Xenos](https://github.com/xenia-project/xenia/blob/master/src/xenia/gpu/xenos.h) and its [command processor](https://github.com/xenia-project/xenia/blob/master/src/xenia/gpu/command_processor.cc).