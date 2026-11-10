# X360 Nexus Android 0.5 — prototipo

O Android carrega uma biblioteca Rust ARM64 via JNI e executa um teste real da instrucao PowerPC `addi` em uma memoria simulada. O teste confirma tambem o contador do kernel experimental. GPU e audio ainda sao apenas estruturas experimentais. **Nao executa jogos Xbox 360, incluindo Forza Horizon.**

## GitHub
Envie o CONTEUDO deste ZIP para a raiz do repositorio (nao coloque dentro de uma pasta adicional). Os caminhos devem ser `core/Cargo.toml`, `core/src/lib.rs`, `android/app/...` e `.github/workflows/android.yml`. Remova pastas acidentais `core/core` quando tiver confirmado que os arquivos certos estao em `core/src`.

Actions > X360 Nexus Android 0.5 > Run workflow. Se o workflow passar, baixe o artefato X360-Nexus-Android-0.5-APK (o GitHub o entrega como ZIP contendo app-debug.apk).

## Limitacoes
A compilacao do APK nao foi executada neste ambiente. JNI e compilacao NDK dependem do workflow passar. O APK inclui apenas biblioteca ARM64; nao suporta dispositivos Android 32-bit.
