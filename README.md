# Project Xenon 4.7 — Kernel Homebrew Engine

Base: versão 4.6, preservando ISO XDVDFS, análise XEX2, memória virtual, controles e interface Mali OpenGL ES.

## Novo
- Interpretador PowerPC experimental com instrução `sc` (`0x44000002`) e ABI **privada** para homebrews originais.
- `r0=1`: retorna contador de ticks em `r3`; `r0=2`: retorna número de páginas em `r3`; `r0=3`: encerra execução com `r3=0`.
- Teste Rust `kernel_demo()` e JNI `nativeKernelSelfTest()`, botão Android **TESTAR KERNEL HOMEBREW**.
- Limite de execução e validação de acesso à memória preservados.

## Limitações
NÃO implementa o kernel Xbox 360, descriptografia XEX, GPU Xenos, nem execução de Minecraft Xbox 360 Edition. As chamadas `sc` são exclusivas deste projeto, não APIs Xbox 360.

## Compilar
`cargo test --manifest-path core/Cargo.toml`

Para APK: envie o conteúdo desta pasta à branch `feature/project-xenon-v4.7` e execute `.github/workflows/android.yml` via GitHub Actions.

Código-fonte fornecido sem APK pré-compilado; não testado em dispositivo neste ambiente.
