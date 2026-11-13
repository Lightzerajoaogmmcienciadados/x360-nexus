# X360: GALAXY v5.6 — XEX2 import validation and PPC boot trace

Incremental source update to Android ARM64 / Rust / C++ emulator research project.

## Changes

- Import inspector preserves type-0 variables and intact type-1 thunks, and has a strictly bounded second path for **whole 16-byte PPC lis/ori/mtctr/bctr veneers** only where the low immediate matches a type-0 ordinal **within the same imported library** and the whole veneer sits in a verified executable range. No arbitrary `.text` patching, no invented imports; diagnostic counts for candidates and proven pairs. This does not guarantee 142 thunk matches for Minecraft.
- Records the first guest instruction (PC, opcode, disassembly, r1/r3/r13/r30/r31 before and after), making the reported `lwz r11,148(r30)` with EA=0x94 actionable without randomly setting r30.
- Initializes only verified xboxkrnl variable ordinals (`KeTimeStampBundle`, `XboxHardwareInfo`, `XboxKrnlVersion`, `XexExecutableModuleHandle`, `ExLoadedCommandLine`, `ExLoadedImageName`, `VdGlobalDevice`, `VdGlobalXamDevice`, `VdGpuClockInMHz`). All values are **simplified offline test models**, not a fully emulated Xbox 360 kernel. Other cells remain zero-filled placeholders.
- Implements bounded KeQueryPerformanceFrequency (50 MHz) and KeQuerySystemTime (guest Big-Endian FILETIME at validated pointer) and keeps failures for unimplemented ordinals, including XAM.
- Existing Android Keystore-protected key handling, immersive interface, virtual controls, AES/Basic/LZX, XDVDFS and PM4 diagnostic renderer are retained.
- Additional synthetic tests for import veneer pairing and rejection of mismatched ordinals, boot first-instruction log, basic exports, kernel time; makes C++ assertions effective in Release CI builds.

## Scope and safety

Not a commercially playable Minecraft emulator. The game title ID 0x4D530A81 was **not supplied for this patch**; no real Minecraft import table has been examined. There is no complete Xbox kernel HLE, no usable Xbox Xenos shader conversion or Vulkan renderer and no guaranteed 142 thunk matches. Neither console keys nor game files are distributed.

To triage a new log, compare `IMPORTS` counts (`candidatos de veneer` and `veneers ligados por ordinal`), `INSTRUCAO #0`, `PC`, `OP`, `EA` and `r30` before/after. The APK builds in `.github/workflows/build-x360-android-apk.yml` on GitHub Actions.