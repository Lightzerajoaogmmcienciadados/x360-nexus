# X360 Nexus / Project Xenon v4.2 — Android ISO/XDVDFS

Uma atualização **incremental** para o projeto Rust/JNI v4.1 já presente em
`Lightzerajoaogmmcienciadados/x360-nexus`. Acrescenta um leitor XDVDFS em C++20,
biblioteca Android JNI própria e interface para listar e copiar arquivos de ISO.

## O que funciona

- Abre via Storage Access Framework (ACTION_OPEN_DOCUMENT) sem permissão de armazenamento geral.
- Faz leitura aleatória, com `pread`, de imagens de vários GiB sem carregá-las inteiras.
- Detecta a assinatura exata **`MICROSOFT*XBOX*MEDIA`** (20 bytes) no setor 32
  em relação aos offsets usuais XISO, XGD2, XGD3, XGD1 e XGD2 hybrid.
- Valida também a segunda assinatura no final do descritor de volume.
- Lê campos little-endian do XDVDFS, incluindo tabela de diretórios como árvore binária,
  caminha recursivamente por subpastas, detecta ciclos, impõe limites de segurança.
- Localiza `default.xex` case-insensitive, preferindo arquivo na raiz.
- Extrai `default.xex` para arquivo escolhido pelo usuário, com leitura em blocos de 64 KiB.
- Copia bytes brutos de `default.xex` diretamente para **RAM emulada esparsa** de 512 MiB
  iniciada em `0x82000000`, com limite seguro de XEX de 128 MiB; imprime CRC32 e
  verifica a assinatura XEX2 na memória.
- Preserva teste PowerPC Rust JNI da v4.1, e cria biblioteca JNI C++ `libx360xdvdfs.so`.

## Limitações importantes

- Carregar um **arquivo XEX bruto** em staging é diferente de carregá-lo como programa
  executável: esta versão não descriptografa nem descompacta conteúdo, não remapeia
  seções XEX na memória de execução, não resolve imports e **não inicia jogos**.
- A RAM esparsa C++ é separada da memória de trabalho da CPU Rust. A integração entre
  esses espaços é uma etapa posterior, não foi apresentada como pronta.
- Não reproduz integralmente o firmware, kernel ou GPU do Xbox 360.
- Algumas ISOs de formatos híbridos atípicos ou provedores Android sem `seek` precisam
  de adaptações. Selecionar a ISO local pelo gerenciador de arquivos costuma funcionar.
- O leitor suporta setores XDVDFS usuais de 2048 bytes e arquivos de até 4 GiB no
  registro; a operação de **staging** limita XEX a 128 MiB e **exportação** a 512 MiB.
- A listagem no Android mostra até 350 arquivos para não sobrecarregar a tela, mas
  os diretórios são percorridos até um limite de 15.000 entradas.

## Como atualizar seu repositório

Copie os caminhos deste pacote para a mesma estrutura no repositório. Os arquivos
`core/` (Rust) e demais componentes anteriores são preservados. Ou use a branch
proposta pelo projeto que contém essas alterações.

Para testar o leitor C++ em Windows/Linux:

```sh
cmake -S . -B build/xdvdfs-host -DCMAKE_BUILD_TYPE=Release
cmake --build build/xdvdfs-host
ctest --test-dir build/xdvdfs-host --output-on-failure
```

Para construir o aplicativo Android ARM64, use `.github/workflows/build-x360-android-apk.yml`.
O workflow instala SDK Android 35, NDK 27.2.12479018, CMake 3.22.1, Rust ARM64 e
Gradle 8.10.2. O build inclui ambas as bibliotecas JNI, Rust e C++.

## Android — como usar

1. Abra o app e escolha **SELECIONAR ISO**.
2. Toque em **ANALISAR ISO E LISTAR ARQUIVOS** para ver caminhos e tamanhos.
3. Toque em **CARREGAR default.xex NA RAM EMULADA** para testar o staging.
4. Opcionalmente toque em **EXTRAIR default.xex PARA ARQUIVO** para salvar o XEX
   em um local que você escolheu.
5. **CONSULTAR MEMORIA RAM EMULADA** exibe informação da última cópia.

Os testes são realizados com ISOs sintéticas geradas em tempo de execução;
o projeto não inclui arquivos de jogos comerciais.

## Estrutura desta atualização

- `android/app/src/main/cpp/xdvdfs.{hpp,cpp}`: leitura, diretórios, RAM esparsa
- `android/app/src/main/cpp/jni_bridge.cpp`: JNI para inspeção, staging, exportação
- `android/app/src/main/cpp/CMakeLists.txt`: compilar `libx360xdvdfs.so`
- `android/app/src/main/java/com/x360nexus/MainActivity.java`: interface Android
- `android/app/build.gradle`: novo alvo C++ no aplicativo
- `android/app/src/main/AndroidManifest.xml`: nome atualizado do aplicativo
- `tests/xdvdfs_tests.cpp`: testes XISO/XGD2/XGD3 e entradas inválidas
- `CMakeLists.txt`: compilação de testes C++ em PC
- `.github/workflows/build-x360-android-apk.yml`: CI Android e validação das 2 bibliotecas

Referência de formato: https://xboxdevwiki.net/XDVDFS
