# Project Xenon 5.1 — Real XEX Loader

Android + Rust, baseado na versão 5.0. Código-fonte completo.

## Novo na 5.1

- Botão **INSPECIONAR XEX REAL** para selecionar um arquivo XEX2 pelo seletor Android.
- JNI transfere um descritor duplicado ao Rust e o fecha automaticamente.
- Leitura limitada a 32 MiB e validação de limites e cabeçalhos.
- Diagnóstico de image base, entry point, imports e formato de armazenamento.
- Identifica arquivos candidatos a homebrew sem criptografia e sem imports.
- Testes unitários do analisador Rust.

## Limites

O botão novo **não executa** um XEX comercial. A v5.0 mantém apenas boot de homebrew sintético. Não há descompressão/descriptografia, resolução real de imports, kernel Xbox 360 completo nem GPU Xenos. Minecraft Xbox 360 não inicia.

## Compilar

1. Envie todo o conteúdo à branch `feature/project-xenon-v5.1`.
2. Execute `.github/workflows/android.yml` pelo GitHub Actions na branch.
3. Baixe o artifact APK e instale no Android.

Para testar localmente: `cargo test --manifest-path core/Cargo.toml`.

**Validação:** arquivo ZIP verificado. Compilação Android e testes Rust não executados neste ambiente.
