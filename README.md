# X360 Nexus / Project Xenon v4.5 — experimental PPC runtime and GPU-aware Android shell

This is a developmental upgrade to v4.4. It **does not run commercial Xbox 360 games**. The Xbox kernel, Xenos command processor, shaders, PPC VMX128, full MMU and real title imports remain unimplemented. Do not interpret successful tests or the presence of Vulkan settings as game compatibility.

## Changes

- Rust PPC64 subset: arithmetic (`addi`, `addis`, `ori`, `oris`, `add`, `or/mr`, `rlwinm`), comparisons (`cmpwi`, `cmplwi`), selected branch conditions (`b`, `bl`, `bc` with CR/CTR, `bclr`, `bcctr`), SPR LR/CTR moves, loads/stores (`lwz`, `lwzu`, `stw`, `stwu`, `lbz`, `stb`, `lhz`, `sth`). All accesses are checked and big-endian.
- Private HLE **test ABI**, not real xboxkrnl/xam ordinals, implements bounded virtual allocation accounting, TLS, simulated thread record, critical section bookkeeping and synthetic video mode query. Unregistered imports remain errors. A secondary stub can return r3=0 only after explicit registration; it logs that no service was implemented.
- A bounded Rust runtime on a separate Android worker: reads a PE snapshot of **at most 16 MiB** from the existing v4.4 mapped C++ guest RAM; starts at its actual XEX entry PC, checks section execute ranges, runs in 8,000-instruction chunks (total cap 2 million), measures genuine IPS and pauses on unknown opcode, `sc` without private ABI, out-of-range fetch or stop. Diagnostics show PC, next opcode, CR, LR, CTR and r3-r10.
- Android dark `activity_main.xml` with selected ISO cards, HUD, start/stop controls, key selection via SAF, preferences, expandable trace.
- EGL OpenGL ES renderer/version probe on a temporary pbuffer and Android Vulkan feature report. No graphics framebuffer for a real Xbox 360 title is presented. Turnip cannot be loaded automatically and is disabled, and other toggles configure preferences for future backends, not a working shader translator.

## Files edited/created (Rust, C++, Kotlin, XML, CMake/Gradle)

- `core/src/cpu.rs`, `core/src/memory.rs`, `core/src/kernel.rs`, `core/src/runtime.rs` (new), `core/src/lib.rs`
- `android/app/src/main/cpp/jni_bridge.cpp`, `gpu_settings.cpp` (new), `CMakeLists.txt`
- `android/app/src/main/java/com/x360nexus/MainActivity.kt`, `GpuDetector.kt` (new)
- `android/app/src/main/res/layout/activity_main.xml` (new)
- `android/app/src/main/res/drawable/nexus_card.xml`, `nexus_button.xml` (new)
- `android/app/src/main/AndroidManifest.xml`, `android/app/build.gradle`
- `.github/workflows/build-x360-android-apk.yml` updated for v4.5 diagnostics/artifact

The C++ reader, XEX AES/LZX and PE mapping from v4.4, and Rust trace from v4.3 are preserved. No console key is included or persisted.

## CI

- `cargo test --manifest-path core/Cargo.toml`
- `cmake -S . -B build/xdvdfs-host && cmake --build build/xdvdfs-host && ctest --test-dir build/xdvdfs-host --output-on-failure`
- Android ARM64 Gradle APK by `.github/workflows/build-x360-android-apk.yml` (SDK 35, NDK 27, Java 17, Kotlin 2.0.21).

## Important constraints

The app displays **FPS: N/D** until actual graphics are being rendered. Only measured **IPS** reflects guest interpreter steps. Vulkan device capabilities are reported by Android; backend setting is a *request*, not evidence of active Vulkan/Xenos rendering. A 16 MiB PE window limit and 2-million-step execution cap protect the device. The process does not have an Xbox kernel or genuine import resolver. For educational tests, the `sc` private ABI uses r0 = `0x58450000 + service_id` (1..10); this is *not* the real Xbox ABI.