# X360: GALAXY — v5.1 (experimental Android CPU / GPU integration)

This is an incremental update over v5.0 of the open-source educational X360 Nexus project. **It does not boot commercial Xbox 360 games.** A successful APK build only proves the source compiles and the automated tests pass.

## New in v5.1

- **Guest CPU:** expanded PowerPC interpreter for select CRT/boot patterns (including `subf`, `cmpw/cmplw`, `andi.`, `rldicl` subset, `ld/std` basic DS form, indexed reads/writes, LR/CTR/CR and basic FPU). Unimplemented instructions still report PC and opcode and pause rather than silently continuing.
- **Continuous native execution:** Rust worker thread executes bounded step batches until stopped or an unsupported operation occurs. The Android UI polls IPS, PC, opcode, and r3–r10 without executing the CPU on the UI thread.
- **Paged image transfer:** Android streams a prepared PE32 from the original C++ XDVDFS/XEX/AES/LZX loader into Rust-owned **sparse 512 MiB guest RAM**, in 64 KiB chunks. Supports PE image sizes up to 128 MiB, replacing the prior 16 MiB Java-side snapshot limit. Host pages are committed on demand and guest accesses enforce page read/write/execute permissions. This is **not** an Xbox 360 TLB/MMU or a shared C++/Rust hardware memory model.
- **HLE test services:** synthetic memory allocations now map/unmap guest pages. The private test ABI and cooperative scheduler are retained. Xbox kernel and XAM commercial ordinals are not fully implemented and will fail diagnostically.
- **PM4:** diagnostic command processor now records `EVENT_WRITE` packets in addition to Type 0/2/3, `SET_CONSTANT`, `REG_RMW`, and observations of draw commands. A custom **test-only** register `0xC000` can change the OpenGL clear color. It is not an Xbox Xenos GPU register or real shader translation.
- **Android:** fixes status-bar inset handling for buttons, adds analog-stick views and more virtual button overlays wired to the experimental Rust JNI pad state. SurfaceView/EGL/OpenGL ES diagnostic output is preserved.

## Loading and testing

The loader sequence remains: local ISO -> XDVDFS -> `default.xex` -> XEX2 -> optional user-provided 16-byte key -> AES-128-CBC -> Basic/LZX -> mapped PE32 -> streamed guest pages -> bounded-ISA PowerPC thread. No console key or games are provided. Unsupported images, imports, instructions and PM4 draw/shader commands stop or report an explicit diagnostic.

To run native C++ unit tests:

```sh
cmake -S . -B build/host -DCMAKE_BUILD_TYPE=Release
cmake --build build/host --parallel 2
ctest --test-dir build/host --output-on-failure
```

To run Rust tests:

```sh
cargo test --manifest-path core/Cargo.toml
```

Android APK via `.github/workflows/build-x360-android-apk.yml`. The artifact is named `X360-Nexus-v5.1-Android-ARM64-Debug-APK`.

## Modified/new files relative to v5.0

- Rust: `core/src/cpu.rs`, `core/src/memory.rs`, `core/src/kernel.rs`, `core/src/runtime.rs`, `core/Cargo.toml`.
- C++: `android/app/src/main/cpp/jni_bridge.cpp`, `xenos_pm4.hpp`, `xenos_pm4.cpp`, `xenos_video.cpp`.
- Kotlin: `android/app/src/main/java/com/x360nexus/MainActivity.kt`, `AnalogPad.kt` (new).
- XML: `android/app/src/main/res/layout/activity_main.xml`.
- Build/test: `android/app/build.gradle`, `.github/workflows/build-x360-android-apk.yml`, `tests/pm4_tests.cpp`.

**Known limitations:** incomplete Xbox PowerPC ISA (including VMX128 and exceptions), incomplete Xbox 360 kernel/XAM imports and file handles, no Xenos shader translation, no Vulkan backend, no automatic game GPU MMIO/DMA interception, and no independently verified commercial game compatibility. The diagnostic GL scene is not game output.