# Project Xenon 4.9 — XEX Image Engine

Baseado no código completo da v4.8. Adiciona `core/src/xex_image.rs` com inspeção limitada do descritor XEX2 `Base file format`, tipo de armazenamento e código de criptografia, com checagem de limites e testes unitários.

## Android
O botão **TESTAR XEX IMAGE ENGINE** executa um teste sintético no núcleo Rust JNI. Não analisa automaticamente a ISO selecionada. O botão de análise da ISO continua disponível como na v4.8.

## Compilar
`cargo test --manifest-path core/Cargo.toml` e GitHub Actions > workflow Android > Run workflow na branch v4.9.

## Limitações
**Não executa Minecraft Xbox 360.** Não inclui descriptografia, descompressão, mapeamento real de código XEX, imports do kernel nem emulação Xenos. A leitura de formato é um diagnóstico; o teste usa dados sintéticos. Código-fonte não compilado/testado neste ambiente.
