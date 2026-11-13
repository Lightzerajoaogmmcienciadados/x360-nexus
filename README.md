# X360: GALAXY v5.5.1 — XEX2 library-table boundary regression fix

This patch corrects the offset of `xex2_import_library` records. Per the open-source Xenia `xex2_opt_import_libraries` structure, the first record starts **exactly at 12 + `string_table.size`**, relative to the optional-header start. The previous parser unconditionally rounded this absolute offset up to a multiple of four, which could skip data for a non-four-byte-aligned table. The updated code also supports an unpadded final library name and rejects unexpected nonzero bytes in the string table.

A dedicated regression test constructs a valid 13-byte string table with its first record at an unaligned address. The test rejects the older implementation and passes with the offset fix.

All other v5.5 features are retained: AndroidKeyStore-protected external XEX key, one-click immersive Android launcher, bounded PowerPC guest memory and HLE diagnostics. **Compatibility with a particular retail title, Xbox kernel imports and real Xenos graphics has not been established**. This is an incremental loader correction, not a playable-game guarantee.

Run `cmake -S . -B build && cmake --build build && ctest --test-dir build --output-on-failure` and `cargo test --manifest-path core/Cargo.toml`. The GitHub Actions workflow builds the Android ARM64 debug APK after host tests pass.



- XEX import parser: reads `0x000103FF` XEX2 import library records as a bounded list, respects string table 4-byte alignment/name count, walks library records by individual record size and checks guest slot values. Validated `type 0` variable records are returned separately from `type 1` function thunks. Unknown `0x41...` etc. remain diagnostics; they are **not automatically treated as imports or overwritten**.
- Rust variable import backing: validated type-0 descriptors receive individually allocated 256-byte cells at guest address `0x72000000+` (read/write, big-endian pointer in the original imported slot). This removes null pointers for *synthetic backing only*. The payload in unknown export cells is zero: **neither Xbox kernel globals nor XEX executable module metadata is fully emulated**; no guarantee `r30` is initialized.
- HLE: still only supports explicitly implemented Xbox ordinals, and stops on unknown calls. No silent success on unimplemented imports.
- One-tap launch: after saving a key through AndroidKeystore AES-GCM, tapping a game entry or choosing an ISO starts XDVDFS → AES → Basic/LZX → PE32 → validated imports → PPC session in the background while an immersive landscape screen opens. The UI offers transparent virtual analogs, D-Pad, A/B/X/Y, LB/RB, LT/RT, Start/Back, log overlay and exit menu. Rendering uses the preexisting diagnostic OpenGL ES SurfaceView and does not display Minecraft graphics.
- Input data is sent through the current Rust JNI test controller bridge. `XamInputGetState` real Xbox ABI remains unimplemented.

## Expected v5.5 test on Minecraft title 0x4D530A81

Compare the new import diagnostics (`variaveis validadas`, `thunks identificados`, `[UNKNOWN]`) and `IMPORT_VARIABLE_CELLS` with v5.4. If import slots still report values that resemble instructions, collect the XEX descriptor/library metadata and PC/EA from the log rather than guessing that every slot is a thunk. We do not bundle any Xbox 360 title, retail key, or system binary.

## Build

- `cmake -S . -B build && cmake --build build && ctest --test-dir build --output-on-failure`
- `cd core && cargo test`
- GitHub Actions `.github/workflows/build-x360-android-apk.yml` builds the debug ARM64 APK (Java 17, Gradle, Rust, NDK).