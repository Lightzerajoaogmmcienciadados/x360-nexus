# Project Xenon 5.0 — XEX Boot Engine

Projeto Android + Rust baseado na versão 4.9. Inclui o módulo `core/src/xex_boot.rs` e botão **TESTAR XEX BOOT HOMEBREW**.

## Implementado
- Carregamento de payload PowerPC de XEX2 sintético, não criptografado, não comprimido e sem imports Xbox.
- Validação de base, entry point, tamanho e alinhamento.
- Mapeamento na memória virtual e execução de quatro instruções de teste com orçamento limitado.
- JNI Android e teste Rust `cargo test --manifest-path core/Cargo.toml`.
- Preservados os módulos de análise XDVDFS/XEX2 e controles da versão 4.9.

## Limitações
**Não inicia Minecraft Xbox 360 nem outros jogos comerciais.** Não há descriptografia, descompressão real de XEX, relocação, imports de kernel Xbox 360, GPU Xenos nem execução de XEX reais complexos. O teste é sintético e não deve ser confundido com compatibilidade de jogos.

## Build
Envie os arquivos do projeto a uma branch `feature/project-xenon-v5.0`, abra GitHub Actions e execute o workflow Android. Não foi possível compilar localmente neste ambiente; compile e valide antes de instalar.
