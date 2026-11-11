# Project Xenon 4.3 — Minecraft XEX Engine

Projeto Android + Rust experimental, baseado no pacote 4.2 Mali Edition.

## Funcionalidade nova
1. Selecione sua ISO XGD2/XDVDFS pelo seletor Android.
2. Toque em **ANALISAR DEFAULT.XEX (MINECRAFT)**.
3. O Rust localiza Default.xex na raiz da imagem, valida limites, lê até 64 KiB do cabeçalho e mostra campos XEX2.

Não extrai o arquivo completo, não descriptografa XEX e **não executa Minecraft Xbox 360 Edition**. A leitura é somente diagnóstico. Algumas variantes de ISO não são suportadas.

## Compilação
Execute `cargo test --manifest-path core/Cargo.toml` e o workflow `.github/workflows/android.yml` no GitHub Actions para gerar o APK. O workflow deve ser executado na branch onde estes arquivos foram enviados.

## Hardware
Interface Android usa OpenGL ES experimental e pode ser testada na Mali-G57 MC2; isto não implementa GPU Xenos.
