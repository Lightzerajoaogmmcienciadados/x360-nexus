# Project Xenon 4.6 — Execution Engine (experimental)

Full source project based on v4.5, with Rust sparse-memory PowerPC execution engine, JNI test button, and Android OpenGL ES demo for Mali GPUs.

## What is implemented
- Executes a **bounded, original PowerPC homebrew test** from mapped virtual memory.
- Instructions: addi, addis, ori, xori, b, add, and a project-specific zero halt sentinel.
- Tests memory access permissions, bounds, unsupported instructions, and infinite-loop execution budget.
- Preserves XDVDFS and XEX metadata diagnostics, virtual controller, Android UI, and Mali-compatible OpenGL ES demo.
- Android button **TESTAR EXECUTION ENGINE** calls Rust JNI.

## What is NOT implemented
- **Minecraft Xbox 360 / commercial game execution is not supported.** XEX image decoding/decryption, imports, relocation, Xbox kernel, GPU Xenos and sound emulation are incomplete.
- The OpenGL ES view is a demo, **not** a Xenos-to-Mali translator.
- This package is source code, not a built or verified APK.

## Build
1. Unzip into a repository branch `feature/project-xenon-v4.6` (preserve folders).
2. GitHub Actions > Project Xenon Galaxy Android > Run workflow > select v4.6 branch.
3. Download the resulting Android APK artifact if the workflow succeeds.
4. Locally run `cargo test --manifest-path core/Cargo.toml` with Rust installed.

The project is not affiliated with Minecraft, Microsoft or Xendroid.
