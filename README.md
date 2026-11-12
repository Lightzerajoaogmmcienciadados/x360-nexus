# X360 Nexus v4.3 — XEX2 parser + PE32 loader + bounded CPU tracing

Incremental educational update of the existing v4.2 Rust/C++ Android application.

## What is implemented

- Keeps XDVDFS ISO listing and `default.xex` raw staging from v4.2.
- Parses XEX2 fixed and optional headers (big-endian), Security Info, File Format Info, Image Base, Entry Point, Title ID, and import-library descriptors (read-only diagnostics).
- Extracts unencrypted, uncompressed XEX payloads and handles XEX Basic block plus zero-fill expansion (hard cap 128 MiB).
- Implements a standalone portable AES-128-CBC primitive tested with standard AES vectors. An already unwrapped **session key** must be supplied by the legitimate test program developer; the Android UI never embeds a console retail key.
- Maps a valid POWERPCBE PE32 basefile to sparse guest RAM according to the section table and checks executable entrypoint; replaces raw staging with mapped PE in shared C++ RAM.
- Kotlin Android interface: **5. EXECUTAR XEX NA CPU (TRACE LIMITADO)**. This button attempts preparation and sends at most 256 bytes from a valid executable section via JNI to the Rust PPC interpreter.
- Rust diagnostic trace: executes up to 32 recognized PPC instructions (`addi`, `addis`, `ori`, `oris`, `add`, `b/bl`), stops at unsupported opcodes/syscalls, and prints PC/LR/CTR and GPR0-GPR7.
- Includes C++ XDVDFS + XEX unit tests and Rust CPU/trace tests. GitHub Actions builds ARM64 APK with Rust and C++ JNI.

## What is NOT implemented

- No bundled retail console keys; no automatic session-key unwrap or commercial XEX decryption. Encrypted files show a diagnostic rather than pretending to execute.
- XEX LZX (normal) is detected and **not decompressed**. Basic and uncompressed formats work under the stated constraints.
- Real xboxkrnl.exe / xam.xex import ordinal resolution and kernel-service emulation are **not implemented**: library names and raw guest slots are enumerated as unresolved imports; no fake successful return codes.
- The Rust CPU receives only a bounded code window copied from C++ guest memory. Branches outside the slice stop; this is not a full cross-language guest MMU bridge.
- No complete Xbox 360 Xenon JIT, Xenos graphics, real firmware/kernel or commercial game execution.

## Main files (C++, Rust, Kotlin)

- `android/app/src/main/cpp/xex_loader.hpp` & `xex_loader.cpp` — metadata, import inventory, PE32 mapping and Basic decompression.
- `android/app/src/main/cpp/xex_crypto.cpp` — AES-128-CBC utility with caller-provided session key.
- `android/app/src/main/cpp/jni_bridge.cpp` — existing ISO/staging functions plus XEX analysis, PE preparation and code-slice read.
- `core/src/xex_trace.rs` & `cpu.rs` & `lib.rs` — bounded interpreter and JNI bridge to Kotlin.
- `android/app/src/main/java/com/x360nexus/MainActivity.kt` — UI replacing the old Java class.
- `tests/xex_loader_tests.cpp` — synthetic XEX + PE fixture, AES test vector and corruption guards.

## Important build configuration

The Android UI is now **Kotlin**, so `android/build.gradle` and `android/app/build.gradle` enable the Kotlin plugin (version 2.0.21), and the old `MainActivity.java` is removed to avoid duplicate Java classes. CMake includes `xex_loader.cpp` and `xex_crypto.cpp` in `libx360xdvdfs.so`; the existing Rust `libx360nexus.so` is retained. CI is `.github/workflows/build-x360-android-apk.yml`.

## Test on PC (C++ only)

```sh
cmake -S . -B build
cmake --build build
ctest --test-dir build --output-on-failure
```

## Use on Android

1. Select a locally accessible ISO using SAF.
2. Analyze ISO, then load raw `default.xex` into staging.
3. Use **ANALISAR CABECALHOS XEX2** to inspect encryption/compression, base and entrypoint.
4. Use **5. EXECUTAR XEX NA CPU (TRACE LIMITADO)**. If protected/LZX, the app explains the blocker. An unencrypted PE32/XEX homebrew fixture can be mapped and stepped up to 32 instructions.

No official Xbox game is bundled; use only files you have permission to inspect.

References: https://github.com/xenia-project/xenia/blob/master/src/xenia/cpu/xex_module.cc and https://github.com/xenia-project/xenia/blob/master/src/xenia/kernel/util/xex2_info.h