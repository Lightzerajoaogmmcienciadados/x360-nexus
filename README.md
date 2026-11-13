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

## X360 GALAXY v5.6 — boot/UI/HLE experimental

- Android: modo paisagem imersivo após selecionar a ISO e recuperar a chave externa pelo Android Keystore; visor com FPS discreto e MENU/LOG, controles em `ConstraintLayout` e respeitando recortes e barras do sistema.
- C++: parser XEX2 `0x000103FF` com indexação de biblioteca, catálogo de thunks por ordinal e stubs PowerPC `sc 2; blr` restritos a seções executáveis verificadas. Diagnóstico `IMPORT_THUNKS` medido no XEX carregado, nunca fixado em 142.
- Rust: boot HLE parcial, variáveis em memória virtual de baixo endereço e `XamInputGetState` ligado aos eixos/botões de toque por JNI, incluindo perfil offline local.
- Testes: inventário sintético de 154 slots de tipo 0 e 142 thunks de tipo 1, e instrução inicial `0x8214AB10`.

**Limitações:** o núcleo ainda não implementa CPU PowerPC/VMX128, Xenos GPU e kernel Xbox 360 integralmente; o Minecraft retail não está confirmado como inicializável ou renderizável nesta versão. Valores de variáveis HLE são modelos de diagnóstico, não drivers/hardware Xbox genuínos. Para validar 142 thunks de um jogo real, consulte o log criado pela própria ISO e seus descriptors; não inferir resultado pelos testes sintéticos. Não são distribuídos jogos ou chaves.
