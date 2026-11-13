# X360: GALAXY v5.3 — XEX Basic/AES automation + import thunk diagnostics

## Added

- Automatic preparation of selected ISO/default.xex on selection and after a valid external 16-byte key is entered, with explicit `PE preparado = true` only after successful AES, Basic/LZX and section mapping.
- C++ decoder of XEX2 imports: distinguishes 32-bit variable records from 16-byte function thunks in the mapped PE. Bounds checks and per-library/ordinal reporting; rejects malformed locations and unsupported libraries.
- JNI thunk catalog transferred to Rust CPU before launch; PowerPC interpreter intercepts known import thunk PC values and reports exact ordinal, library, PC and opcode on unsupported functions.
- Only narrow `xboxkrnl.exe` TLS ordinals 338/340/341 delegate to existing single-thread HLE. **This does not imply commercial ABI accuracy or multi-thread TLS fidelity.** Other imports stop explicitly; `r3=0` is never fabricated for unknown calls.
- The existing guest PM4 parser, Android EGL surface, Rust CPU worker, AES and Basic zero-fill decompressor are preserved.
- CTest for synthetic XEX import records; Android ARM64 CI updated to v5.3.

## Not implemented

- 296 fully working xboxkrnl/xam exports (276 + 20 are **import-table records**, not automatically callable functions).
- CPU instruction completeness, real Xbox kernel thread/file/MMIO behavior, Xenos shaders, EDRAM, real textures/geometry or gameplay of Minecraft.
- No retail keys or game data distributed. User-provided encrypted key remains in Android Keystore.

Use legal test executables; CI verifies build, not retail game compatibility.