# Project Xenon 4.5 — Memory Engine

Base: pacote Project Xenon 4.4 XEX Loader. Contém todos os arquivos do projeto Android/Rust, mais `core/src/virtual_memory.rs`.

## Implementado nesta versão
- Memória virtual esparsa por páginas de 4 KiB (até 16 MiB).
- Permissões de leitura, escrita e execução; detecção de páginas sobrepostas.
- Verificação de limites e escrita atômica (sem alteração parcial quando o acesso é inválido).
- Leitura big-endian de instruções PowerPC inclusive cruzando páginas.
- Testes Rust e botão Android **TESTAR MEMÓRIA VIRTUAL** via JNI.
- Mantida a análise XDVDFS e XEX2 da v4.4.

## Compilação
1. Envie os arquivos preservando a estrutura de pastas para sua branch GitHub.
2. Execute `.github/workflows/android.yml` pelo GitHub Actions.
3. Baixe o artifact APK após um build bem-sucedido.

## Limites
A memória virtual implementada é um modelo próprio para homebrew/testes, **não** o MMU do Xbox 360. O `Default.xex` comercial não é descriptografado, descompactado nem executado. A GPU Xenos e APIs do kernel Xbox 360 permanecem não implementadas. Não foi compilado ou testado como APK neste ambiente.
