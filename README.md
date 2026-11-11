# Project Xenon 4.8 — XEX Runtime Diagnostics

Projeto Android + Rust baseado no 4.7, com todos os arquivos de origem.

## Novo
- `core/src/xex_runtime.rs`: análise de pré-requisitos de execução XEX2 com verificações de cabeçalho e lista explícita de bloqueios.
- JNI `nativeRuntimeSelfTest`, botão Android `TESTAR XEX RUNTIME`.
- Versão Android 4.8.0.
- Testes unitários para cabeçalho sintético e dados inválidos.

## O que **não** faz
Não descriptografa nem descomprime executáveis comerciais, não carrega suas seções reais, não resolve imports Xbox, não emula GPU Xenos e **não inicia Minecraft Xbox 360 Edition**. O diagnóstico de runtime usa um cabeçalho sintético para testar o parser. A análise de ISO/XEX2 da versão anterior permanece.

## Compilação
Execute `cargo test --manifest-path core/Cargo.toml` e depois `.github/workflows/android.yml` no GitHub Actions (workflow_dispatch). Não foi possível compilar o APK neste ambiente.

## Android
Mantenha os arquivos nas mesmas pastas. A interface OpenGL ES continua experimental para Mali-G57; isso não é tradução da GPU Xenos.
