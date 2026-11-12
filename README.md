# X360: GALAXY — v5.0 (núcleo experimental integrável)

**Importante:** este projeto NÃO executa jogos comerciais do Xbox 360. Uma
compilação do APK não comprova compatibilidade com Minecraft, Forza etc.

## Auditoria da base v4.5

| Subsistema | Situação encontrada | Atualização v5.0 |
|---|---|---|
| ISO e XEX2 | XDVDFS no C++; AES-CBC + LZX + PE32 sob limites; chave externa de 16 bytes | Preservados. Não inclui chaves de console ou jogos |
| RAM e MMU | Staging C++ esparso de 512 MiB, mas runtime Rust limitado a snapshot contíguo de 16 MiB | Permissões de páginas no snapshot; módulo esparso Rust com R/W/X para testes, ainda não conectado à CPU |
| CPU PowerPC | Interpretador com instruções inteiras e branches; sem FPU e formas indexadas | Acrescenta operações escalares FPU e cargas/lojas indexadas; erros de opcode continuam explícitos |
| Threads | Modelo só registra ID | Escalonamento cooperativo round-robin de tarefas originais de teste; não são os 6 hardware threads do Xenon |
| Kernel | HLE via ABI privada; ordinais reais não mapeados | Eventos, alocação de memória física sintética, modos de vídeo e swaps de teste. NtOpenFile/NtReadFile deixam explícita a ausência de handles de filesystem |
| GPU | Preferências de backend sem renderer nem CP | Analisador PM4 Type 0/2/3 com buffer limitado e EGL/GLES para triângulo diagnóstico na SurfaceView |
| Controles | Estado de controle Rust por JNI disponível | Overlay A/B/esquerda/direita e eventos de controle físico para o estado experimental; XamInputGetState real ainda não implementado |

## Arquitetura

`ISO -> XDVDFS -> default.xex -> XEX2 header -> Session key externa (se exigida) -> AES-CBC -> Basic/LZX -> PE32 -> RAM C++/seções -> snapshot Rust (até 16 MiB) -> CPU PPC interpreter + HLE de teste -> telemetria`.

O pipeline **não é garantido** para todo XEX real: hashes, formatos variantes,
imports/relocações e comportamento do kernel podem impedir a execução.
Os bytes PE/XEX são armazenados segundo sua codificação de origem; operações
PowerPC big-endian são decodificadas explicitamente, e cabeçalhos PE32 são
little-endian. Não se deve inverter os bytes de um PE inteiro.

`CPU Rust -> memória snapshot` e `CPU/C++ -> RAM XDVDFS` **ainda não
compartilham uma MMU única**; o snapshot é limitado a 16 MiB. O módulo
`core/src/mmu.rs` valida um espaço virtual esparso 512 MiB, mas ainda não
substitui a memória de execução do CPU.

`Memória C++ -> nativeProcessPm4(address, words) -> RingBuffer FIFO limitado ->
Type0 / Type2 / Type3 (SET_CONSTANT, REG_RMW, DRAW_INDX)`.
**Ainda não há interceptação automática de MMIO/DMA do jogo**, nem tradução de
shader Xenos. PM4_DRAW_INDX é detectado e marcado como **não implementado**.

`SurfaceView -> videoWorker -> ANativeWindow -> EGL OpenGL ES 3 -> glClear /
triângulo sintético -> eglSwapBuffers`. A superfície mostra um teste com shader
GLSL próprio, **não a saída de um jogo**. Vulkan permanece seleção não implementada,
e Turnip não está incluído ou carregado. FPS reportado refere-se somente aos
quadros de demonstração.

## Testes

C++ host:
```
cmake -S . -B build/host -DCMAKE_BUILD_TYPE=Release
cmake --build build/host
ctest --test-dir build/host --output-on-failure
```

Rust: `cargo test --manifest-path core/Cargo.toml`.

Android: executar `.github/workflows/build-x360-android-apk.yml` no GitHub
Actions. Compila C++ JNI, Rust JNI e Kotlin. O APK fica no artifact
**X360-Nexus-v5.0-Android-ARM64-Debug-APK**.

## Modificações desta versão

- Rust: `cpu.rs`, `memory.rs`, `kernel.rs`, `runtime.rs`, `scheduler.rs` (novo),
  `mmu.rs` (novo), `lib.rs`, `Cargo.toml`.
- C++: `xenos_pm4.{hpp,cpp}` (novos), `xenos_video.{hpp,cpp}` (novos),
  `gpu_pm4_jni.cpp` (novo), `jni_bridge.cpp`, `gpu_settings.cpp`, `CMakeLists.txt`.
- Kotlin/XML: `MainActivity.kt`, `activity_main.xml`, `AndroidManifest.xml`,
  `android/app/build.gradle`.
- Testes: `tests/pm4_tests.cpp`, `CMakeLists.txt` na raiz; workflow atualizado.

Limitações: sem CPU Xenon completa/VMX128/JIT, 6 hardware threads, imports
`xboxkrnl.exe`/`xam.xex` reais, memória partilhada plena, GPU Xenos ou
renderização de jogos. Chaves de console não são armazenadas nem distribuídas.