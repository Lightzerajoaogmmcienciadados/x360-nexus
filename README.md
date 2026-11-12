# Project Xenon 4.4 — AES-128 with externally supplied key + CAB LZX

This incremental source update builds on v4.3 without replacing the XDVDFS
ISO reader or the Rust PowerPC diagnostic tracer.

## New C++ files / changes
- `android/app/src/main/cpp/xex_lzx.cpp`: independently written CAB-LZX
  decoder (uncompressed, Verbatim, Aligned blocks; Huffman lengths and LZ
  back-references) and XEX Normal outer block/chunk framing with SHA-1 checks.
- `xex_loader.hpp/.cpp`: XEX Security Info ImageKey extraction, AES key
  unwrapping with a 16-byte *caller-supplied* external key, AES-CBC image
  decryption, LZX Normal decompression, and existing PE32 section mapping.
- `jni_bridge.cpp`: Android JNI methods to load/discard a key in process RAM;
  the existing `nativePrepareStagedXex` uses it when encryption is present.
- `MainActivity.kt`: new `CARREGAR CHAVE XEX (keys.bin)` and `DESCARTAR
  CHAVE XEX` actions, and the existing bounded `5. EXECUTAR XEX NA CPU` action.
- CMake, Gradle and GitHub Actions updated to compile the new C++ source.

A key file must contain **exactly 16 raw bytes**. No console keys are included,
stored on disk, shown in diagnostics, or fetched from third-party services.
The app discards the key when the user presses the discard button or closes
its Activity; native process memory may still contain copies unless securely
cleared by the operating system.

## Limits and validation
- The LZX decoder uses the classic CAB format, not LZX Delta. Unit tests
  cover uncompressed blocks, Verbatim literals, Aligned literals, framing,
  SHA-1 integrity, malformed data and an AES-128 test vector.
- Intel E8-transformed LZX streams are explicitly rejected (PowerPC PE32
  content should not use Intel x86 fixups). No known commercial game is used
  in tests and real-world XEX compatibility is not guaranteed.
- Memory limits: 128 MiB decompressed payload, 128 MiB XEX input, 512 MiB
  sparse guest address space. Stream is decoded into host memory and mapped
  into a new guest image, not executed in-place from raw staging.
- Parsing and decryption do **not** imply that Xbox 360 games can run. Imports
  `xboxkrnl` / `xam`, CPU ISA completeness, virtual memory, GPU and kernel
  services remain missing. Rust trace still runs at most 32 supported PPC
  instructions from an executable section and stops on unknown instructions.

## Build / test C++

```
cmake -S . -B build -DCMAKE_BUILD_TYPE=Debug
cmake --build build --parallel 2
ctest --test-dir build --output-on-failure
```

Android APK build uses `.github/workflows/build-x360-android-apk.yml` with
Rust ARM64, C++20 JNI and Kotlin. No proprietary XEX data is distributed.