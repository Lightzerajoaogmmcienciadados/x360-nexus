# Project Xenon 4.4 — XEX Loader Diagnostics (Android + Rust)

**Base:** código-fonte v4.3 preservado e estendido.

## Novo na 4.4
- Leitura segura dos registros de cabeçalhos opcionais XEX2 em Rust.
- Detecção de entry point e image base declarados, quando presentes.
- Diagnóstico de chaves de cabeçalhos, validação de limites e testes unitários.
- Interface Android atualizada, JNI da v4.3 preservado.
- Workflow GitHub Actions incluído para compilação do APK Android arm64.

## Como gerar o APK
1. Extraia o ZIP e envie todos os arquivos ao repositório na branch desejada.
2. Em GitHub Actions, execute o workflow **X360 Nexus Android Rust JNI** na mesma branch.
3. Baixe o artefato APK da execução concluída.

## Limites
Não há descriptografia, descompressão, relocação, mapeamento de memória executável, implementação de imports Xbox 360 nem execução de Minecraft.
O número chamado `header_size` na versão anterior é o offset de imagem XEX, não um tamanho de cabeçalho confirmado.
OpenGL ES 2.0 é usado para a interface; **não** traduz comandos da GPU Xenos para Mali.
Este ZIP é código-fonte e não um APK.
