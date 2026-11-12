# MC360 Android — protótipo
Base de pesquisa em Rust + Kotlin + Java + C++ para Android. **Não é um port do Minecraft Xbox 360 Edition e não executa jogos comerciais.**
Inclui importação segura de um arquivo de até vários GB via seletor do Android (leitura de apenas 4096 bytes), reconhecimento inicial de cabeçalhos XEX2/STFS e demo gráfica original OpenGL ES 2.0 com comandos de controle.

## Compilar APK
No GitHub, o workflow `.github/workflows/mc360-android.yml` compila `MC360-Android-Demo-APK` e gera `app-debug.apk`.
A ação aceita tanto execução automática em push quanto manual.

## Limites
Não possui emulação PowerPC Xenon, kernel Xbox, execução XEX, conversão de shaders Xenos, áudio real nem dados ou direitos sobre Minecraft. O arquivo de 7,83 GB não é convertido automaticamente em jogo Android. Não distribua XEX, imagens de jogos ou recursos comerciais.