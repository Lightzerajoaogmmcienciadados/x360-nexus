# X360: GALAXY v5.7 — PowerPC CR, import veneers e Xenos PM4

Atualização incremental do projeto experimental Android ARM64 em Rust/C++.

## Novidades verificáveis na base de código

- CPU: o desvio `bc` (primary opcode 16) já existia na v5.6; agora o disassembler mostra BO/BI/CRbit/taken/target. Interpretação de mcrf, CR lógico (crand/cror/crxor), comparação com SO, fcmpu/fcmpo, rldicl/rldicr e mais operações integer/rotate, sem alegar ISA Xenon/VMX128 completa.
- XEX2: inspeção bounded do código PPC para veneers com as quatro instruções `lis r11; lwz r11,slot(r11); mtctr r11; bctr`. Um thunk só recebe o stub `sc 2; blr` se seu alvo apontar a um slot tipo 0 validado da *mesma* biblioteca. O teste sintético contém 142 correspondências; o jogo real pode apresentar um número diferente.
- HLE: preserva KeTls, memória virtual, contagem/timers, vídeo parcial e XamInputGetState. Falhas para APIs não implementadas continuam visíveis, em vez de retornar sucesso fictício.
- PM4: mantém decodificação de Type 0/2/3, SET_CONSTANT, LOAD_ALU_CONSTANT, DRAW_INDX, DRAW_INDX_2, REG_RMW e EVENT_WRITE; capta uploads de microcódigo IM_LOAD/IM_LOAD_IMMEDIATE, lendo buffers da RAM Rust por JNI.
- `VdGetSystemCommandBuffer` inicia o rastreamento de gravações reais na área de ring buffer emulado; `VdSwap` publica a faixa tocada (até 4096 DWORDs) para o coletor JNI. Isso **não implementa** MMIO do CP, ponteiros de ring ou o protocolo físico completo.
- EGL/OpenGL ES: tenta 3.2, fallback ES3, mantém SurfaceView preta antes de pacotes gráficos válidos; triângulo verde e clear animado de teste removidos.
- Testes: CPU de branches/CR, import IAT sintético, PM4 e shader upload. CI compila APK Android ARM64 e executa testes host Rust/C++.

## Escopo e limitações

**Não há shader translator Xenos ISA → GLSL ES 3.20 funcional, rasterizador EDRAM, VMX128 ou kernel Xbox integral nesta versão.** A recepção de comandos e a captura de microcódigo são estágios preparatórios para o futuro backend; draws permanecem não renderizados. Não há garantia de Minecraft 0x4D530A81 rodando ou de `IMPORT_THUNKS=142` em uma ISO real. O teste comercial descrito pelo usuário não fornece o próprio XEX para análise. Não fazemos bypass de direitos autorais, e nenhuma chave ou jogo é distribuído.

Os logs acessíveis pelo botão MENU/LOG permitem comparar a instrução #0, CR/CTR, branches tomados, inventário de imports e a primeira falha sem fabricar estados HLE.

---

## Histórico: v5.6

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
