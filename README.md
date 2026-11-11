# Project Xenon 4.1: Galaxy — código experimental

Este ZIP é um **overlay** para a branch `feature/project-xenon-v4.0` do repositório x360-nexus. Não é um APK nem um emulador capaz de executar jogos comerciais.

## Incluído
- `core/src/loader.rs`: identificação de ISO XDVDFS, leitura conservadora da árvore de diretório raiz, metadados XEX2.
- `core/src/galaxy.rs`: execução de programa PowerPC de demonstração com limite de passos.
- `core/src/galaxy_jni.rs`: JNI para leitura de ISO, controles e demo.
- `core/src/lib.rs`: integração do núcleo existente.
- `android/.../MainActivity.java`: interface Galaxy, seletor ISO, controles virtuais, gamepad físico, preferências.
- `android/.../GalaxySurface.java`: renderização OpenGL ES 2.0 demonstrativa, **não** GPU Xenos.
- `android/app/build.gradle`: versão 4.1.0.
- `core/Cargo.toml`: versão 4.1.0.

## Instalação
1. Faça backup da branch v4.0 ou crie `feature/project-xenon-v4.1-galaxy`.
2. Copie os arquivos do ZIP para os mesmos caminhos do repositório, preservando `core/src/cpu.rs`, `memory.rs`, `gpu.rs`, `audio.rs`, `kernel.rs` e `input.rs` existentes.
3. Execute `cargo test --manifest-path core/Cargo.toml`.
4. Execute o workflow Android da branch (adicione a branch nova ao filtro de push se criar uma).
5. Baixe o artifact APK da execução bem-sucedida.

## Limitações
- ISO: reconhece certos layouts e lista a raiz; não executa jogo nem implementa todas as variações de XGD.
- XEX: metadados do cabeçalho apenas; sem decriptação, relocação ou execução.
- Kernel: preserva o kernel experimental já existente.
- Mali-G57 MC2: a demo usa OpenGL ES 2.0 amplamente disponível; não há tradução de comandos Xenos nem otimização comprovada para essa GPU.
- Controles: botões, dois sticks e gatilhos enviam estado ao Rust; nenhum jogo comercial consome esse estado ainda.
- Ícone Galaxy: necessita aplicação do recurso visual ao manifesto Android; não incluído neste overlay.
- Código fornecido como ponto de partida e não testado em APK real.

## Ícone original em forma de X
`android/app/src/main/res/drawable/galaxy_icon.xml` é um ícone vetorial original em forma de X (não o logotipo oficial Xbox). Para usá-lo, configure `android:icon="@drawable/galaxy_icon"` no elemento `<application>` do `AndroidManifest.xml` existente. O fundo galáxia é desenhado diretamente pela `MainActivity`.

## Atualização do pacote completo
Este ZIP agora inclui também os módulos Rust de base (cpu, memory, kernel, gpu, audio, input), AndroidManifest com ícone, configuração Gradle e workflow Android. Pode ser colocado na raiz do repositório, preservando outros arquivos existentes.

**Atenção:** não foi possível executar cargo/Gradle neste ambiente. A compilação precisa ser validada pelo GitHub Actions. A demo PowerPC foi corrigida para executar três instruções sequenciais.
