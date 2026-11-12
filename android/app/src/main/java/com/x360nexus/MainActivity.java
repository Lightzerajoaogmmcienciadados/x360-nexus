package com.x360nexus;

import android.app.Activity;
import android.os.Bundle;
import android.os.ParcelFileDescriptor;
import android.content.Intent;
import android.content.Context;
import android.database.Cursor;
import android.graphics.Color;
import android.graphics.Bitmap;
import android.graphics.BitmapFactory;
import android.view.Choreographer;
import android.widget.ImageView;
import android.widget.Toast;
import android.widget.Switch;
import android.os.Vibrator;
import android.os.VibrationEffect;
import android.os.VibratorManager;
import android.graphics.Typeface;
import android.net.Uri;
import android.provider.OpenableColumns;
import android.view.Gravity;
import android.view.InputDevice;
import android.view.KeyEvent;
import android.view.MotionEvent;
import android.view.View;
import android.view.WindowManager;
import android.widget.Button;
import android.widget.EditText;
import android.text.TextWatcher;
import android.text.Editable;
import android.widget.FrameLayout;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.SeekBar;
import android.widget.TextView;
import android.media.AudioTrack;
import android.media.AudioFormat;
import android.media.AudioManager;
import org.json.JSONArray;
import org.json.JSONObject;
import java.io.InputStream;
import java.io.ByteArrayOutputStream;
import android.view.WindowInsets;
import android.os.Build;
import java.io.IOException;
import java.util.Arrays;
import java.util.ArrayList;
import java.util.HashSet;
import java.io.File;
import java.util.Locale;

/**
 * X360 Nexus native Android shell. No HTML, browser, JavaScript or WebView.
 * Experimental PowerPC diagnostics, GLES2, PCM output, Android gamepad,
 * file/library inspection. NOT capable of running retail Xbox 360 games.
 */
public final class MainActivity extends Activity {
    private static final int PICK_GAME = 360;
    private static final int PICK_COVER = 361;
    private static final int PICK_COVERS = 362;
    private static final int PICK_NATIVE_PPC = 363;
    private String pendingCoverGameUri;
    private static final int BG = Color.rgb(6,15,11);
    private static final int PANEL = Color.rgb(15,35,25);
    private static final int BORDER = Color.rgb(40,78,53);
    private static final int GREEN = Color.rgb(85,225,128);
    private static final int LIGHT = Color.rgb(236,248,238);
    private static final int MUTED = Color.rgb(155,179,161);
    private static final int[] PAD_MASKS = {1,2,4,8,16,32,64,128,256,512};
    private static final String[] TABS = {"Início","Biblioteca","Núcleo","Controles","Sobre"};
    private static final boolean RUST_AVAILABLE;
    static {
        boolean ok;
        try { System.loadLibrary("x360nexus");ok=true; }
        catch(Throwable e){ok=false;}
        RUST_AVAILABLE=ok;
    }

    private native String nativeSelfTest();
    private native String nativeSubsystemDiagnostics();
    private native String nativeGpuBenchmark(int frames);
    private native String nativeInspectPackage(byte[] header);
    private native String nativeRunDemo();
    private native int[] nativeGpuLabFrame(int mode);
    private native String nativeGpuLabReport(int mode);
    private native String nativeInspectIso(int fileDescriptor);
    private native String nativeInspectXex(byte[] bytes);
    private native String nativeExecuteRawPpc(byte[] bytes);
    private native void nativeArcadeReset();
    private native int[] nativeArcadeFrame(int buttons,int deltaMs);
    private native String nativeArcadeStatus();
    private native void nativeSetButton(int mask, boolean pressed);
    private native void nativeSetAxis(int axis,int value);
    private native String nativePadStatus();
    private native int nativeGenerateAudio(short[] samples,int frequency,int rate);

    private LinearLayout shell, content, tabs;
    private ScrollView scrollView;
    private GalaxySurface surface;
    private TextView padStatus;
    private int tabIndex = 0;
    private volatile boolean audioPlaying = false;
    private volatile int toneFrequency = 220;
    private Thread audioThread;
    private AudioTrack audioTrack;
    private final Object audioGuard = new Object();
    private boolean arcadeActive=false,arcadeUp=false,arcadeDown=false;
    private float arcadeAxis=0f;
    private long arcadeLastNanos=0;
    private int arcadeFrames=0;
    private Bitmap arcadeBitmap;
    private ImageView arcadeImage;
    private TextView arcadeOutput;
    private Choreographer.FrameCallback arcadeCallback;

    private void vibrate(int durationMs) {
        Haptics.pulse(this,shell,durationMs,false);
    }
    private int dp(float x) {return (int)(getResources().getDisplayMetrics().density*x+.5f);}
    private TextView text(String value,float size,int color,boolean bold) {
        TextView v=new TextView(this);
        v.setText(value);v.setTextColor(color);v.setTextSize(size);
        v.setLineSpacing(dp(2),1.05f);
        if(bold)v.setTypeface(Typeface.DEFAULT,Typeface.BOLD);
        return v;
    }
    private LinearLayout linear(boolean horizontal) {
        LinearLayout v=new LinearLayout(this);
        v.setOrientation(horizontal?LinearLayout.HORIZONTAL:LinearLayout.VERTICAL);
        return v;
    }
    private void add(LinearLayout parent,View child,int topDp) {
        LinearLayout.LayoutParams lp=new LinearLayout.LayoutParams(-1,-2);
        lp.topMargin=dp(topDp);parent.addView(child,lp);
    }
    private LinearLayout panel(String label,String subtitle) {
        LinearLayout p=linear(false);p.setPadding(dp(17),dp(16),dp(17),dp(16));
        android.graphics.drawable.GradientDrawable bg=new android.graphics.drawable.GradientDrawable();
        bg.setColor(PANEL);bg.setCornerRadius(dp(18));bg.setStroke(dp(1),BORDER);
        p.setBackground(bg);
        if(label!=null) add(p,text(label,19,LIGHT,true),0);
        if(subtitle!=null) add(p,text(subtitle,13,MUTED,false),7);
        return p;
    }
    private Button button(String label) {
        Button b=new Button(this);
        b.setText(label);b.setTextColor(BG);b.setAllCaps(false);
        b.setTextSize(14);b.setTypeface(Typeface.DEFAULT,Typeface.BOLD);
        android.graphics.drawable.GradientDrawable bg=new android.graphics.drawable.GradientDrawable();
        bg.setColor(GREEN);bg.setCornerRadius(dp(12));
        b.setBackground(bg);b.setPadding(dp(10),dp(10),dp(10),dp(10));
        b.setOnTouchListener((view,event)->{
            if(event.getActionMasked()==MotionEvent.ACTION_DOWN)Haptics.pulse(this,b,50,false);
            return false;
        });
        return b;
    }
    private Button outlined(String label) {
        Button b=button(label);b.setTextColor(LIGHT);
        android.graphics.drawable.GradientDrawable bg=new android.graphics.drawable.GradientDrawable();
        bg.setColor(Color.rgb(25,56,37));bg.setCornerRadius(dp(12));bg.setStroke(dp(1),BORDER);
        b.setBackground(bg);return b;
    }
    private void status(String title,String value) {
        LinearLayout p=panel(title,value);add(content,p,12);
    }
    private void heading(String title,String desc) {
        add(content,text(title,27,LIGHT,true),6);
        add(content,text(desc,14,MUTED,false),7);
    }
    @Override public void onCreate(Bundle state){
        super.onCreate(state);
        getWindow().setStatusBarColor(BG);
        getWindow().setNavigationBarColor(BG);
        getWindow().clearFlags(WindowManager.LayoutParams.FLAG_TRANSLUCENT_STATUS);
        shell=linear(false);shell.setBackgroundColor(BG);
        // Android 15+ draws content edge-to-edge: reserve both system-bar insets
        // so the title and bottom navigation never sit beneath gesture/status bars.
        if (Build.VERSION.SDK_INT >= 30) {
            getWindow().setDecorFitsSystemWindows(false);
            shell.setOnApplyWindowInsetsListener((view, windowInsets) -> {
                android.graphics.Insets bars =
                    windowInsets.getInsets(WindowInsets.Type.systemBars());
                view.setPadding(bars.left,bars.top,bars.right,bars.bottom);
                return windowInsets;
            });
        } else {
            shell.setFitsSystemWindows(true);
        }
        LinearLayout top=linear(false);top.setPadding(dp(18),dp(17),dp(18),dp(12));
        add(top,text("X360  NEXUS",21,GREEN,true),0);
        add(top,text("NATIVE  •  EXPERIMENTAL  •  RUST / ANDROID",11,MUTED,true),4);
        shell.addView(top,new LinearLayout.LayoutParams(-1,-2));

        scrollView=new ScrollView(this);
        scrollView.setFillViewport(true);
        scrollView.setVerticalScrollBarEnabled(false);
        content=linear(false);
        content.setPadding(dp(18),dp(8),dp(18),dp(18));
        scrollView.addView(content);
        shell.addView(scrollView,new LinearLayout.LayoutParams(-1,0,1f));

        tabs=linear(true);tabs.setBackgroundColor(Color.rgb(22,54,35));
        tabs.setPadding(dp(5),dp(9),dp(5),dp(9));
        shell.addView(tabs,new LinearLayout.LayoutParams(-1,-2));
        setContentView(shell);
        showTab(0);
    }
    private void showTab(int index) {
        stopArcade();
        if(surface!=null){surface.onPause();surface=null;}
        tabIndex=index;content.removeAllViews();tabs.removeAllViews();
        for(int i=0;i<TABS.length;i++) {
            final int selected=i;
            TextView b=text(TABS[i],12,i==index?GREEN:Color.rgb(209,228,211),i==index);
            b.setGravity(Gravity.CENTER);b.setPadding(dp(2),dp(16),dp(2),dp(16));
            b.setOnClickListener(v->showTab(selected));
            LinearLayout.LayoutParams lp=new LinearLayout.LayoutParams(0,-2,1f);
            tabs.addView(b,lp);
        }
        switch(index){
            case 0:home();break;
            case 1:library();break;
            case 2:core();break;
            case 3:controllers();break;
            default:about();break;
        }
        // Changing tab must not preserve the previous page's scroll position.
        if(scrollView!=null)scrollView.post(()->scrollView.scrollTo(0,0));
    }
    private void home() {
        heading("Projeto Xenon","Interface nativa Android, com código Rust integrado via JNI.");
        LinearLayout picture=panel("X360 NEXUS — VISUAL ORIGINAL",
            "Referência visual reutilizada da captura anterior da sua biblioteca, preservando o tema preto e verde.");
        add(content,picture,12);
        try(InputStream ref=getAssets().open("images/x360-nexus-anterior.jpg")){
            Bitmap previous=BitmapFactory.decodeStream(ref);
            if(previous!=null){
                ImageView preview=new ImageView(this);
                preview.setImageBitmap(previous);
                preview.setAdjustViewBounds(true);
                preview.setScaleType(ImageView.ScaleType.FIT_CENTER);
                add(picture,preview,12);
            }
        }catch(Exception unavailable){
            add(picture,text("Tema anterior preservado.",12,MUTED,false),9);
        }
        LinearLayout independent=panel("NEXUS CORE — SEM XENIA",
            "Abra um jogo original jogável feito em Rust. Esta engine NÃO executa ISOs comerciais do Xbox 360.");
        add(content,independent,14);
        Button arcade=button("▶ Jogar Nexus Paddle (Rust independente)");
        add(independent,arcade,12);
        arcade.setOnClickListener(v->showNativeArcade());
        LinearLayout banner=panel("MOTOR NATIVO  •  ARM64",
            RUST_AVAILABLE?"Biblioteca Rust carregada":"Biblioteca Rust indisponível");
        add(content,banner,18);
        surface=new GalaxySurface(this);
        LinearLayout.LayoutParams glp=new LinearLayout.LayoutParams(-1,dp(230));
        glp.topMargin=dp(14);content.addView(surface,glp);
        add(content,text("Visualização OpenGL ES 2.0 original • NÃO é a GPU Xenos",12,MUTED,false),10);
        LinearLayout diag=panel("DIAGNÓSTICO DO EMULADOR",
            "CPU Xenon: subconjunto PowerPC\nMemória virtual: regiões R/W/X experimentais\nGPU: framebuffer Rust + OpenGL ES 2.0\nÁudio: PCM sintetizado via Rust/AudioTrack\nKernel: threads e ciclos experimentais\nISO/XEX2: análise de cabeçalhos opcionais\nControles: tela sensível ao toque + gamepad Android");
        add(content,diag,16);
        Button run=button("Executar diagnóstico CPU + GPU + kernel");
        add(diag,run,14);
        TextView output=text("Pronto para testar o núcleo.",13,LIGHT,false);add(diag,output,10);
        run.setOnClickListener(v->{
            if(!RUST_AVAILABLE){output.setText("Erro: biblioteca nativa ausente.");return;}
            try{output.setText(nativeRunDemo());}catch(Throwable e){output.setText("Erro JNI: "+e);}
        });
        boolean bundledEngine=ExternalXboxBackend.isIntegrated(this);
        LinearLayout emulator=panel(bundledEngine
            ?"XENIA MALI — MOTOR NATIVO INTEGRADO"
            :"XBOX 360 REAL — MOTOR XENDROID MALI EXTERNO",
            bundledEngine
            ?"Motor C++/JIT ARM64/Vulkan do XenDroid Mali presente neste APK. A compatibilidade varia por GPU e jogo."
            :"Integração com emulador Xenia/XenDroid instalado separadamente. Esta edição não contém o motor.");
        add(content,emulator,18);
        add(emulator,text(ExternalXboxBackend.diagnostic(this),12,LIGHT,false),10);
        Button emulatorSite=outlined(bundledEngine
            ?"Abrir configurações completas Xenia Mali"
            :"Instalar motor XenDroid Mali");
        add(emulator,emulatorSite,11);
        emulatorSite.setOnClickListener(v->ExternalXboxBackend.openProject(this));
        Button go=outlined("Abrir biblioteca de jogos");add(content,go,18);
        go.setOnClickListener(v->showTab(1));
    }
    private void core() {
        heading("Núcleo Rust / Xenon","Ferramentas executadas localmente no aparelho.");
        LinearLayout homebrew=panel("EXECUÇÃO DIRETA — NÚCLEO RUST PRÓPRIO",
            "Motor independente do Xenia: demonstração jogável Nexus Paddle e interpretador PowerPC limitado para programas próprios .ppcbin. Não inicia Minecraft/Forza ISO.");
        add(content,homebrew,11);
        Button playNative=button("▶ Executar jogo nativo Rust");
        add(homebrew,playNative,10);
        playNative.setOnClickListener(v->showNativeArcade());
        Button runPpc=outlined("Executar programa PowerPC próprio (.ppcbin)");
        add(homebrew,runPpc,9);
        runPpc.setOnClickListener(v->{
            Intent pick=new Intent(Intent.ACTION_OPEN_DOCUMENT);
            pick.addCategory(Intent.CATEGORY_OPENABLE);
            pick.setType("*/*");
            pick.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION);
            startActivityForResult(pick,PICK_NATIVE_PPC);
        });
        add(homebrew,text("Programas PowerPC: código bruto big-endian de até 64 KiB, com até 20 mil instruções. Apenas execução isolada de teste.",12,MUTED,false),7);
        addRefreshControls(content);
        LinearLayout benchmarkPanel=panel("GPU LAB v10.0 — BENCHMARK NATIVO",
            "Teste 30 quadros com 24 triângulos texturizados por quadro, Z-buffer e estatísticas reais do rasterizador Rust. Não é GPU Xenos real.");
        add(content,benchmarkPanel,12);
        Button benchButton=button("Executar benchmark GPU (30 frames)");
        add(benchmarkPanel,benchButton,10);
        TextView benchOutput=text("Aguardando benchmark.",12,LIGHT,false);
        add(benchmarkPanel,benchOutput,10);
        benchButton.setOnClickListener(v->{
            if(!RUST_AVAILABLE){benchOutput.setText("Núcleo Rust indisponível.");return;}
            benchButton.setEnabled(false);
            benchOutput.setText("Executando GPU experimental...");
            new Thread(()->{
                String report;
                try{report=nativeGpuBenchmark(30);}
                catch(Throwable err){report="Erro GPU: "+err.getMessage();}
                final String output=report;
                runOnUiThread(()->{benchOutput.setText(output);benchButton.setEnabled(true);});
            }).start();
        });
        LinearLayout subsystem=panel("COMPAT LAB 6.6 — DIAGNÓSTICO AMPLIADO",
            "Execute testes do interpretador PowerPC, escalonador experimental, mixer de áudio PCM e GPU 3D.");
        add(content,subsystem,13);
        Button runAll=button("Executar diagnóstico de todos os subsistemas");
        add(subsystem,runAll,10);
        TextView subsystemStatus=text("Pronto para testar os módulos nativos.",12,LIGHT,false);
        add(subsystem,subsystemStatus,12);
        runAll.setOnClickListener(v->{
            if(!RUST_AVAILABLE){subsystemStatus.setText("Biblioteca Rust indisponível");return;}
            try{subsystemStatus.setText(nativeSubsystemDiagnostics());}
            catch(Throwable err){subsystemStatus.setText("Falha no diagnóstico: "+err.getMessage());}
        });
        status("CPU PowerPC experimental",
            "Instruções: addi/addis, mulli, subfic, rlwinm, loads/stores com atualização, lha, shifts, branches via LR/CTR, comparações e operações inteiras.\nAinda faltam MMU, VMX128, SMT, JIT e grande parte da ISA.");
        status("Kernel e memória","Memória big-endian e regiões virtuais 4 KiB R/W/X; threads com estados Ready/Waiting/Stopped e escalonamento cooperativo. Não é o kernel Xbox 360.");
        status("GPU","Framebuffer RGBA em Rust + cena OpenGL ES 2.0 no Android. Sem tradução de shaders ou comandos Xenos.");
        LinearLayout xenos=panel("XENOS LAB v6.5 — GPU EXPERIMENTAL",
            "Além das cenas v6.4: Z-buffer, teste de profundidade, triângulos indexados, cores interpoladas, filtragem bilinear de textura e cubo 3D projetado. Rasterizador Rust, não GPU Xenos real.");
        add(content,xenos,12);
        ImageView gpuPreview=new ImageView(this);
        gpuPreview.setScaleType(ImageView.ScaleType.FIT_CENTER);
        gpuPreview.setBackgroundColor(Color.rgb(3,10,7));
        LinearLayout.LayoutParams previewParams=new LinearLayout.LayoutParams(-1,dp(192));
        previewParams.topMargin=dp(12);
        xenos.addView(gpuPreview,previewParams);
        TextView gpuInfo=text("Selecione uma cena para testar o pipeline de desenho.",12,MUTED,false);
        add(xenos,gpuInfo,8);
        String[] labels={"Pista","Triângulos","Texturas","Z-buffer","Gouraud","Cubo 3D"};
        for(int row=0;row<2;row++){
            LinearLayout gpuModes=linear(true);
            for(int column=0;column<3;column++){
                final int selectedMode=row*3+column;
                Button option=outlined(labels[selectedMode]);
                option.setTextSize(12);
                LinearLayout.LayoutParams op=new LinearLayout.LayoutParams(0,dp(55),1f);
                op.setMargins(dp(2),0,dp(2),0);
                gpuModes.addView(option,op);
                option.setOnClickListener(v->renderGpuLab(selectedMode,gpuPreview,gpuInfo));
            }
            add(xenos,gpuModes,row==0?12:9);
        }
        if(RUST_AVAILABLE)renderGpuLab(5,gpuPreview,gpuInfo);
        status("Análise XEX2","Leitura da tabela opcional (chave e valor) e localização do Default.xex dentro da ISO. O módulo comercial não é iniciado.");
        LinearLayout audio=panel("ÁUDIO NATIVO","Aperte para ouvir um sinal de diagnóstico sintetizado pelo núcleo Rust.");
        add(content,audio,12);
        TextView frequency=text("Frequência: "+toneFrequency+" Hz",13,LIGHT,false);
        add(audio,frequency,11);
        SeekBar hz=new SeekBar(this);
        hz.setMax(700);hz.setProgress(toneFrequency-100);add(audio,hz,6);
        hz.setOnSeekBarChangeListener(new SeekBar.OnSeekBarChangeListener(){
            @Override public void onProgressChanged(SeekBar b,int p,boolean fromUser){
                toneFrequency=p+100;
                frequency.setText("Frequência: "+toneFrequency+" Hz");
            }
            @Override public void onStartTrackingTouch(SeekBar b){}
            @Override public void onStopTrackingTouch(SeekBar b){}
        });
        Button toggle=button(audioPlaying?"Parar áudio":"Tocar teste de áudio");
        add(audio,toggle,13);
        TextView info=text(audioPlaying?"Em reprodução":"48 kHz • PCM 16-bit • Mono",12,MUTED,false);
        add(audio,info,8);
        toggle.setOnClickListener(v->{
            if(audioPlaying){stopSound();toggle.setText("Tocar teste de áudio");info.setText("Parado");}
            else if(RUST_AVAILABLE){info.setText("Iniciando AudioTrack...");toggle.setText("Parar áudio");startSound(info,toggle);}
            else {info.setText("Falha: núcleo Rust não carregado");}
        });
        LinearLayout cpu=panel("AUTOTESTES RUST","Executa instruções PowerPC e verifica framebuffer, áudio FIFO e ticks do kernel.");
        add(content,cpu,12);
        Button test=outlined("Executar testes");add(cpu,test,12);
        TextView out=text("Aguardando diagnóstico.",13,LIGHT,false);add(cpu,out,10);
        test.setOnClickListener(v->{
            if(!RUST_AVAILABLE){out.setText("Biblioteca nativa não encontrada.");return;}
            try{out.setText(nativeSelfTest());}catch(Throwable e){out.setText("Erro JNI: "+e);}
        });
    }
    private void renderGpuLab(int mode,ImageView target,TextView report){
        if(!RUST_AVAILABLE){report.setText("Núcleo Rust indisponível.");return;}
        try{
            int[] data=nativeGpuLabFrame(mode);
            if(data==null||data.length!=320*180)throw new IllegalStateException("Buffer GPU inválido");
            Bitmap bitmap=Bitmap.createBitmap(data,320,180,Bitmap.Config.ARGB_8888);
            target.setImageBitmap(bitmap);
            report.setText(nativeGpuLabReport(mode));
        }catch(Throwable ex){
            report.setText("Erro no GPU LAB: "+ex.getMessage());
        }
    }

    private void applyLibraryFilter(ArrayList<View> views,ArrayList<String> names,
        ArrayList<Boolean> favorites,String search,boolean onlyFavorites){
        String query=search==null?"":search.toLowerCase(Locale.ROOT).trim();
        for(int k=0;k<views.size();k++){
            boolean found=names.get(k).contains(query);
            boolean favorite=favorites.get(k);
            views.get(k).setVisibility((found && (!onlyFavorites||favorite))?View.VISIBLE:View.GONE);
        }
    }

    private void library() {
        heading("Biblioteca de jogos",
            "Nexus Core próprio para homebrew experimental e motor Xenia integrado preservado para jogos Xbox 360.");
        LinearLayout independent=panel("JOGOS DIRETOS — MOTOR X360 NEXUS (SEM XENIA)",
            "Execute a demonstração Nexus Paddle em Rust sem abrir o Xenia. Para ISOs/XEX comerciais, o motor Rust ainda não tem compatibilidade.");
        add(content,independent,12);
        Button direct=button("▶ Iniciar jogo nativo sem Xenia");
        add(independent,direct,10);
        direct.setOnClickListener(v->showNativeArcade());
        Button profiles=outlined("Gerenciar perfis Xbox 360 locais");
        add(independent,profiles,9);
        profiles.setOnClickListener(v->showXboxProfiles());
        LinearLayout pick=panel("ISO / XEX / STFS / GOD / XBLA",
            "Seus arquivos permanecem no celular. O núcleo Rust lê metadados; o motor Xenia integrado tenta executar jogos compatíveis.");
        add(content,pick,15);
        Button importGame=button("+ Adicionar jogo ou arquivo");
        add(pick,importGame,12);
        boolean xeniaIntegrated=ExternalXboxBackend.isIntegrated(this);
        LinearLayout xboxBackend=panel("EXECUÇÃO DE JOGOS — MOTOR XENIA",
            xeniaIntegrated
                ?"Inicie jogos no próprio X360 Nexus, pelo processo nativo Xenia ARM64/Vulkan já incluído. Permissão de arquivos do Android necessária."
                :"Motor Xenia externo: instale o XenDroid Mali para executar arquivos ISO/XEX.");
        add(content,xboxBackend,13);
        TextView backendState=text(ExternalXboxBackend.diagnostic(this),12,LIGHT,false);
        add(xboxBackend,backendState,9);
        Button installBackend=outlined(xeniaIntegrated?"Abrir configurações do Xenia Mali":"Obter XenDroid Mali no GitHub");
        add(xboxBackend,installBackend,9);
        installBackend.setOnClickListener(v->ExternalXboxBackend.openProject(this));
        if(xeniaIntegrated){
            Button players=outlined("Gerenciar perfil de jogador Xbox 360");
            add(xboxBackend,players,8);
            players.setOnClickListener(v->ExternalXboxBackend.openProject(this));
        }
        Button checkBackend=outlined("Atualizar diagnóstico do motor");
        add(xboxBackend,checkBackend,8);
        checkBackend.setOnClickListener(v->backendState.setText(ExternalXboxBackend.diagnostic(this)));
        if(xeniaIntegrated){
            Button permission=button(ExternalXboxBackend.hasRequiredStorageAccess(this)
                ?"Permissão de arquivos autorizada"
                :"Autorizar arquivos para execução de jogos");
            add(xboxBackend,permission,10);
            permission.setOnClickListener(v->ExternalXboxBackend.requestStorageAccess(this));
            add(xboxBackend,text("O Xenia requer a permissão especial do Android. "
                +"Você concede essa autorização no sistema; não é ativada automaticamente.",12,MUTED,false),8);
        }
        importGame.setOnClickListener(v->{
            Intent open=new Intent(Intent.ACTION_OPEN_DOCUMENT);
            open.addCategory(Intent.CATEGORY_OPENABLE);
            open.setType("*/*");
            open.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION|Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION);
            startActivityForResult(open,PICK_GAME);
        });

        LinearLayout covers=panel("CAPAS DOS JOGOS",
            "JPG, PNG ou WebP. Nomeie a imagem como o jogo ou como o Title ID (8 caracteres hexadecimais). O cache fica no app e funciona sem internet.");
        add(content,covers,14);
        Button bulk=outlined("Importar capas automaticamente");
        add(covers,bulk,12);
        bulk.setOnClickListener(v->chooseCovers());
        add(covers,text("Exemplos: Forza Horizon.jpg • 4D530805.png",12,MUTED,false),9);

        LinearLayout organizer=panel("ORGANIZAR JOGOS",
            "Pesquise por nome, organize o catálogo e marque seus favoritos.");
        add(content,organizer,14);
        EditText search=new EditText(this);
        search.setSingleLine(true);
        search.setHint("Pesquisar jogo...");
        search.setTextColor(LIGHT);
        search.setHintTextColor(MUTED);
        search.setTextSize(15);
        add(organizer,search,6);
        Switch onlyFav=new Switch(this);
        onlyFav.setText("Mostrar somente favoritos");
        onlyFav.setTextColor(LIGHT);
        onlyFav.setTextSize(14);
        add(organizer,onlyFav,8);
        boolean nameSort=getPreferences(MODE_PRIVATE).getBoolean("sortByName",false);
        Button sortBtn=outlined(nameSort?"Ordenação: nome A–Z":"Ordenação: adição");
        add(organizer,sortBtn,9);
        sortBtn.setOnClickListener(v->{
            getPreferences(MODE_PRIVATE).edit().putBoolean("sortByName",!nameSort).apply();
            showTab(1);
        });

        JSONArray games=loadLibrary();
        ArrayList<JSONObject> listing=new ArrayList<>();
        for(int i=0;i<games.length();i++){
            JSONObject item=games.optJSONObject(i);
            if(item!=null)listing.add(item);
        }
        if(nameSort)listing.sort((a,b)->a.optString("name","")
            .compareToIgnoreCase(b.optString("name","")));
        ArrayList<View> cardViews=new ArrayList<>();
        ArrayList<String> cardNames=new ArrayList<>();
        ArrayList<Boolean> cardFavorites=new ArrayList<>();
        add(content,text("Arquivos adicionados: "+games.length(),18,LIGHT,true),20);
        if(games.length()==0){
            add(content,panel("Biblioteca vazia",
                "Adicione uma ISO ou XEX. A capa poderá ser selecionada depois; não é preciso alterar o jogo original."),12);
        }
        for(JSONObject item:listing) {
            String name=item.optString("name","Arquivo");
            String uri=item.optString("uri","");
            String scan=item.optString("scan","Ainda não inspecionado");
            long bytes=item.optLong("size",-1);
            String titleId=item.optString("titleId","");
            if(titleId.isEmpty())titleId=titleIdFromScan(scan);
            LinearLayout card=panel(null,null);
            add(content,card,13);
            cardViews.add(card);
            cardNames.add(name.toLowerCase(Locale.ROOT));
            cardFavorites.add(item.optBoolean("favorite",false));
            final int cardIndex=cardViews.size()-1;
            LinearLayout summary=linear(true);
            FrameLayout poster=new FrameLayout(this);
            android.graphics.drawable.GradientDrawable posterBackground=
                new android.graphics.drawable.GradientDrawable(
                    android.graphics.drawable.GradientDrawable.Orientation.TL_BR,
                    new int[]{Color.rgb(22,77,46),Color.rgb(4,19,12)});
            posterBackground.setCornerRadius(dp(12));
            posterBackground.setStroke(dp(1),GREEN);
            poster.setBackground(posterBackground);
            File cachedCover=CoverStore.find(this,uri,name,titleId);
            Bitmap coverBitmap=CoverStore.preview(cachedCover);
            if(coverBitmap!=null){
                ImageView photo=new ImageView(this);
                photo.setImageBitmap(coverBitmap);
                photo.setScaleType(ImageView.ScaleType.CENTER_CROP);
                photo.setContentDescription("Capa de "+name);
                poster.addView(photo,new FrameLayout.LayoutParams(-1,-1));
            } else {
                TextView empty=text("X360\nNEXUS\n\nSEM CAPA",13,GREEN,true);
                empty.setGravity(Gravity.CENTER);
                empty.setTextAlignment(View.TEXT_ALIGNMENT_CENTER);
                poster.addView(empty,new FrameLayout.LayoutParams(-1,-1));
            }
            LinearLayout.LayoutParams posterParams=new LinearLayout.LayoutParams(dp(98),dp(139));
            posterParams.rightMargin=dp(14);
            summary.addView(poster,posterParams);
            LinearLayout details=linear(false);
            details.addView(text(name,18,LIGHT,true));
            add(details,text(formatBytes(bytes),13,MUTED,false),8);
            if(!titleId.isEmpty())add(details,text("Title ID: "+titleId,12,GREEN,false),8);
            String shortScan=scan.split("\n",2)[0];
            add(details,text(shortScan,12,MUTED,false),8);
            summary.addView(details,new LinearLayout.LayoutParams(0,-2,1f));
            add(card,summary,0);

            Button choose=outlined(cachedCover==null?"Adicionar capa":"Trocar capa");
            add(card,choose,13);
            choose.setOnClickListener(v->chooseCoverForGame(uri));
            if(ExternalXboxBackend.supportsFile(name)){
                final String gameTitleId=titleId;
                Button play=button("▶ Iniciar jogo no X360 Nexus");
                add(card,play,11);
                play.setOnClickListener(v->{
                    String result=ExternalXboxBackend.play(this,Uri.parse(uri),name,gameTitleId);
                    Toast.makeText(this,result,Toast.LENGTH_LONG).show();
                });
                if(ExternalXboxBackend.isIntegrated(this)){
                    Button optimize=outlined("⚙ Perfis de jogo — Xenia Manager");
                    add(card,optimize,10);
                    optimize.setOnClickListener(v->
                        showMaliPerformanceSettings(uri,name,gameTitleId));
                }
            }
            Button favorite=outlined(item.optBoolean("favorite",false)
                ?"★ Favorito — desmarcar":"☆ Adicionar aos favoritos");
            add(card,favorite,10);
            favorite.setOnClickListener(v->{
                boolean newValue=!cardFavorites.get(cardIndex);
                cardFavorites.set(cardIndex,newValue);
                JSONArray library=loadLibrary();
                for(int k=0;k<library.length();k++){
                    JSONObject entry=library.optJSONObject(k);
                    if(entry!=null && uri.equals(entry.optString("uri"))){
                        try{entry.put("favorite",newValue);}catch(Exception ignored){}
                    }
                }
                saveLibrary(library);
                favorite.setText(newValue?"★ Favorito — desmarcar":"☆ Adicionar aos favoritos");
                applyLibraryFilter(cardViews,cardNames,cardFavorites,
                    search.getText().toString(),onlyFav.isChecked());
            });
            TextView fullReport=text(scan,12,MUTED,false);
            fullReport.setVisibility(View.GONE);
            Button showReport=outlined("Ver detalhes técnicos");
            add(card,showReport,10);
            add(card,fullReport,9);
            showReport.setOnClickListener(v->{
                boolean opening=fullReport.getVisibility()!=View.VISIBLE;
                fullReport.setVisibility(opening?View.VISIBLE:View.GONE);
                showReport.setText(opening?"Ocultar detalhes":"Ver detalhes técnicos");
                vibrate(12);
            });
            if(name.toLowerCase(Locale.ROOT).endsWith(".ppcbin")
                || name.toLowerCase(Locale.ROOT).endsWith(".nxpp")
                || name.toLowerCase(Locale.ROOT).endsWith(".ppc")) {
                Button execute=button("Executar código PowerPC experimental");
                add(card,execute,10);
                execute.setOnClickListener(v->runRawPpc(Uri.parse(uri),card));
            }
            Button remove=outlined("Remover da biblioteca");
            add(card,remove,10);
            remove.setOnClickListener(v->{
                JSONArray all=loadLibrary(),next=new JSONArray();
                for(int k=0;k<all.length();k++){
                    JSONObject entry=all.optJSONObject(k);
                    if(entry!=null&&!uri.equals(entry.optString("uri")))next.put(entry);
                }
                saveLibrary(next);
                CoverStore.removeGame(this,uri);
                vibrate(24);
                showTab(1);
            });
        }
        search.addTextChangedListener(new TextWatcher(){
            @Override public void beforeTextChanged(CharSequence s,int start,int count,int after){}
            @Override public void onTextChanged(CharSequence s,int start,int before,int count){
                applyLibraryFilter(cardViews,cardNames,cardFavorites,s.toString(),onlyFav.isChecked());
            }
            @Override public void afterTextChanged(Editable e){}
        });
        onlyFav.setOnCheckedChangeListener((b,checked)->{
            applyLibraryFilter(cardViews,cardNames,cardFavorites,search.getText().toString(),checked);
        });
        add(content,panel("EXECUÇÃO DOS JOGOS",
            "O aplicativo detecta ISOs XGD2/XGD3, localiza default.xex, inspeciona XEX2 e Title ID. Capas são apenas imagens locais, não habilitam execução de jogos."),15);
    }

    private static String titleIdFromScan(String report) {
        java.util.regex.Matcher m=java.util.regex.Pattern.compile(
            "(?im)^Title ID:\\s*([0-9a-f]{8})\\s*$").matcher(report);
        return m.find()?m.group(1).toUpperCase(Locale.ROOT):"";
    }

    /**
     * Enables a XenDroid per-title graphics profile before the next game boot.
     * Existing per-title TOML is backed up before the first modification.
     * No game ISO or save data is touched.
     */
    private void showMaliPerformanceSettings(String gameUri,String gameName,String titleId){
        if(!ExternalXboxBackend.isIntegrated(this)){
            Toast.makeText(this,"Motor Xenia não integrado neste APK",Toast.LENGTH_LONG).show();
            return;
        }
        if(MaliPerformance.validTitleId(titleId)){
            showMaliPerformanceOptions(gameName,titleId.toUpperCase(Locale.ROOT));
            return;
        }
        EditText entry=new EditText(this);
        entry.setSingleLine(true);
        entry.setHint("Ex.: 584111F7");
        entry.setTextColor(LIGHT);
        entry.setHintTextColor(MUTED);
        entry.setInputType(android.text.InputType.TYPE_CLASS_TEXT
            |android.text.InputType.TYPE_TEXT_FLAG_CAP_CHARACTERS);
        new android.app.AlertDialog.Builder(this)
            .setTitle("Title ID do jogo")
            .setMessage("Esse arquivo não informou o Title ID. Consulte os detalhes do jogo e insira o identificador real de 8 dígitos hexadecimais.")
            .setView(entry)
            .setNegativeButton("Cancelar",null)
            .setPositiveButton("Continuar",(d,which)->{
                String parsed=entry.getText().toString().trim().toUpperCase(Locale.ROOT);
                if(!MaliPerformance.validTitleId(parsed)){
                    Toast.makeText(this,"Title ID inválido. Não alterei as configurações.",
                        Toast.LENGTH_LONG).show();
                    return;
                }
                JSONArray library=loadLibrary();
                for(int k=0;k<library.length();k++){
                    JSONObject item=library.optJSONObject(k);
                    if(item!=null && gameUri.equals(item.optString("uri"))){
                        try{item.put("titleId",parsed);}catch(Exception ignored){}
                    }
                }
                saveLibrary(library);
                showMaliPerformanceOptions(gameName,parsed);
            }).show();
    }

    private void showMaliPerformanceOptions(String gameName,String titleId){
        final String[] choices={
            "Desempenho — 1x, sem pós-processamento (Mali)",
            "Equilibrado — 2x, texturas padrão",
            "Qualidade — 2x, FXAA e CAS",
            "Personalizado — escolher resolução",
            "Limite de FPS Xenia (0 / 30 / 60 / 90 / 120)",
            "Restaurar configurações originais",
            "Abrir gerenciador completo do Xenia"
        };
        boolean backedUp=MaliPerformance.isManaged(this,titleId);
        String selected=MaliPerformance.currentProfile(this,titleId);
        new android.app.AlertDialog.Builder(this)
            .setTitle("Xenia Manager • "+gameName)
            .setMessage("Title ID: "+titleId+"\nPerfil atual: "+selected+
                "\nBackup: "+(backedUp?"disponível":"nenhum")+
                "\n\nAs alterações são feitas apenas neste jogo e entram em vigor "
                +"na próxima inicialização. A compatibilidade e o FPS variam.")
            .setItems(choices,(dialog,which)->{
                if(which==6){
                    ExternalXboxBackend.openProject(this);
                }else if(which==5){
                    runMaliProfile(titleId,null,1);
                }else if(which==4){
                    showXeniaFrameLimit(titleId);
                }else if(which==3){
                    showCustomMaliProfile(gameName,titleId);
                }else{
                    String[] ids={"performance","balanced","quality"};
                    runMaliProfile(titleId,ids[which],1);
                }
            })
            .setNegativeButton("Cancelar",null)
            .show();
    }

    private void showXeniaFrameLimit(String titleId){
        final int[] caps={0,30,60,90,120};
        final String[] choices={"Sem limite adicional (0 FPS)","30 FPS","60 FPS",
            "90 FPS experimental","120 FPS experimental"};
        new android.app.AlertDialog.Builder(this)
            .setTitle("Limite de FPS do Xenia")
            .setMessage("Esta configuração é um limite no renderizador Vulkan do Xenia, "
                +"NÃO força 90/120 FPS nem altera a lógica interna de jogos de 60 FPS. "
                +"Taxas altas podem piorar desempenho/temperatura no Galaxy A17. "
                +"Para Minecraft, 30 ou 60 é mais conservador.")
            .setItems(choices,(d,which)->new Thread(()->{
                String msg=MaliPerformance.setFrameLimit(this,titleId,caps[which]);
                runOnUiThread(()->new android.app.AlertDialog.Builder(this)
                    .setTitle("Limite de FPS")
                    .setMessage(msg)
                    .setPositiveButton("OK",null).show());
            },"xenia-game-fps").start())
            .setNegativeButton("Cancelar",null).show();
    }

    private void showCustomMaliProfile(String gameName,String titleId){
        LinearLayout layout=linear(false);
        layout.setPadding(dp(20),dp(12),dp(20),dp(8));
        TextView resolution=text("Resolução interna: 1x",14,LIGHT,true);
        add(layout,resolution,8);
        SeekBar scale=new SeekBar(this);
        scale.setMax(2);
        scale.setProgress(0);
        add(layout,scale,10);
        add(layout,text("1x: mais leve para Mali. 2x e 3x exigem mais GPU. "
            +"Ajustes são exclusivos deste Title ID.",12,MUTED,false),8);
        scale.setOnSeekBarChangeListener(new SeekBar.OnSeekBarChangeListener(){
            @Override public void onProgressChanged(SeekBar bar,int value,boolean fromUser){
                resolution.setText("Resolução interna: "+(value+1)+"x");
            }
            @Override public void onStartTrackingTouch(SeekBar bar){}
            @Override public void onStopTrackingTouch(SeekBar bar){}
        });
        new android.app.AlertDialog.Builder(this)
            .setTitle("Perfil personalizado • "+gameName)
            .setView(layout)
            .setNegativeButton("Cancelar",null)
            .setPositiveButton("Salvar perfil",(dialog,which)->
                runMaliProfile(titleId,"custom",scale.getProgress()+1))
            .show();
    }

    private void runMaliProfile(String titleId,String profile,int scale){
        new Thread(()->{
            String message=(profile==null)
                ?MaliPerformance.restore(this,titleId)
                :MaliPerformance.apply(this,titleId,profile,scale);
            runOnUiThread(()->{
                new android.app.AlertDialog.Builder(this)
                    .setTitle("Configurações do jogo")
                    .setMessage(message)
                    .setPositiveButton("OK",null)
                    .show();
            });
        },"x360-xenia-game-profile").start();
    }

    private void chooseCoverForGame(String gameUri){
        pendingCoverGameUri=gameUri;
        Intent pick=new Intent(Intent.ACTION_OPEN_DOCUMENT);
        pick.addCategory(Intent.CATEGORY_OPENABLE);
        pick.setType("image/*");
        pick.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION);
        startActivityForResult(pick,PICK_COVER);
    }

    private void chooseCovers(){
        pendingCoverGameUri=null;
        Intent pick=new Intent(Intent.ACTION_OPEN_DOCUMENT);
        pick.addCategory(Intent.CATEGORY_OPENABLE);
        pick.setType("image/*");
        pick.putExtra(Intent.EXTRA_ALLOW_MULTIPLE,true);
        pick.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION);
        startActivityForResult(pick,PICK_COVERS);
    }

    private String displayName(Uri uri){
        try(Cursor cursor=getContentResolver().query(uri,null,null,null,null)){
            if(cursor!=null && cursor.moveToFirst()){
                int index=cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME);
                if(index>=0&&!cursor.isNull(index))return cursor.getString(index);
            }
        }catch(Exception ignored){}
        return uri.getLastPathSegment();
    }

    private void processCoverPicker(int request,Intent data){
        final String selectedGame=pendingCoverGameUri;
        pendingCoverGameUri=null;
        final ArrayList<Uri> images=new ArrayList<>();
        HashSet<String> seen=new HashSet<>();
        if(data.getClipData()!=null){
            for(int i=0;i<data.getClipData().getItemCount();i++){
                Uri uri=data.getClipData().getItemAt(i).getUri();
                if(uri!=null&&seen.add(uri.toString()))images.add(uri);
            }
        }
        if(data.getData()!=null&&seen.add(data.getData().toString()))images.add(data.getData());
        if(images.isEmpty())return;
        vibrate(18);
        new Thread(()->{
            int saved=0;
            String warning=null;
            for(Uri uri:images){
                try{
                    String name=displayName(uri);
                    if(!CoverStore.supportedName(name)){
                        warning="Use arquivos JPG, PNG ou WebP";
                        continue;
                    }
                    String key;
                    if(request==PICK_COVER && selectedGame!=null)
                        key=CoverStore.gameKey(selectedGame);
                    else
                        key=CoverStore.imageAlias(name);
                    CoverStore.save(this,uri,key);
                    saved++;
                }catch(Throwable err){
                    warning=String.valueOf(err.getMessage());
                }
            }
            final int completed=saved;
            final String problem=warning;
            runOnUiThread(()->{
                if(completed>0)vibrate(27);
                String msg=completed+" capa(s) adicionada(s)";
                if(problem!=null)msg+=" • "+problem;
                Toast.makeText(this,msg,Toast.LENGTH_LONG).show();
                if(tabIndex==1)showTab(1);
            });
        },"x360-offline-cover-cache").start();
    }
    /** Optional homebrew format: bare, big-endian, 4-byte PowerPC instructions. */
    private void runRawPpc(Uri file,LinearLayout card) {
        TextView output=text("Executando programa PPC isolado...",13,LIGHT,false);
        add(card,output,8);
        new Thread(()->{
            String report;
            try {
                if(!RUST_AVAILABLE)throw new IOException("Nucleo Rust indisponivel");
                ByteArrayOutputStream data=new ByteArrayOutputStream();
                try(InputStream input=getContentResolver().openInputStream(file)){
                    if(input==null)throw new IOException("Nao foi possivel abrir arquivo");
                    byte[] temp=new byte[4096];
                    int n;
                    while((n=input.read(temp))!=-1){
                        if(n==0)continue;
                        if(data.size()+n>65536)throw new IOException("Codigo PPC limitado a 64 KB");
                        data.write(temp,0,n);
                    }
                }
                report=nativeExecuteRawPpc(data.toByteArray());
            }catch(Throwable err){report="Erro: "+err.getMessage();}
            final String message=report;
            runOnUiThread(()->output.setText(message));
        },"x360-ppc-sandbox").start();
    }

    private JSONArray loadLibrary() {
        try {
            String value=getPreferences(MODE_PRIVATE).getString("library","[]");
            return new JSONArray(value);
        } catch(Exception e){return new JSONArray();}
    }
    private void saveLibrary(JSONArray data) {
        getPreferences(MODE_PRIVATE).edit().putString("library",data.toString()).apply();
    }
    private String formatBytes(long bytes) {
        if(bytes<0)return "Tamanho desconhecido";
        if(bytes<1024)return bytes+" B";
        if(bytes<1024*1024)return String.format(Locale.US,"%.1f KB",bytes/1024f);
        if(bytes<1024L*1024*1024)return String.format(Locale.US,"%.1f MB",bytes/(1024f*1024f));
        return String.format(Locale.US,"%.2f GB",bytes/(1024.0*1024*1024));
    }
    private void controllers() {
        heading("Controles","Toque nos botões ou conecte um controle USB/Bluetooth Android.");
        addRefreshControls(content);
        LinearLayout profileSection=panel("PERFIS DE JOGADOR (OFFLINE)",
            "Crie, liste e selecione perfis locais do motor Xenia integrado sem entrar no Xbox Live.");
        add(content,profileSection,12);
        Button playerProfiles=button("Gerenciar perfis Xbox 360");
        add(profileSection,playerProfiles,9);
        playerProfiles.setOnClickListener(v->showXboxProfiles());
        LinearLayout p=panel("GAMEPAD XINPUT EXPERIMENTAL","A, B, X, Y, D-pad, Start, Back e gatilhos virtuais.");
        add(content,p,14);
        padStatus=text(RUST_AVAILABLE?nativePadStatus():"Núcleo Rust não carregado",13,LIGHT,false);
        add(p,padStatus,8);
        String[][] rows={{"↑","←","↓","→"},{"X","Y","A","B"},{"LB","RB","Start","Back"}};
        int[][] codes={{1,2,4,8},{64,128,16,32},{256,512,1024,2048}};
        for(int row=0;row<rows.length;row++){
            LinearLayout line=linear(true);
            for(int col=0;col<4;col++){
                Button b=outlined(rows[row][col]);final int mask=codes[row][col];
                LinearLayout.LayoutParams lp=new LinearLayout.LayoutParams(0,dp(58),1f);
                lp.leftMargin=dp(2);lp.rightMargin=dp(2);line.addView(b,lp);
                b.setOnTouchListener((view,event)->{
                    switch(event.getActionMasked()){
                        case MotionEvent.ACTION_DOWN:
                            press(mask,true);return true;
                        case MotionEvent.ACTION_CANCEL:
                        case MotionEvent.ACTION_UP:
                            press(mask,false);return true;
                    }
                    return false;
                });
            }
            add(p,line,12);
        }
        add(p,text("Eixo horizontal esquerdo",13,LIGHT,false),18);
        SeekBar axis=new SeekBar(this);
        axis.setMax(200);axis.setProgress(100);add(p,axis,10);
        axis.setOnSeekBarChangeListener(new SeekBar.OnSeekBarChangeListener(){
            @Override public void onProgressChanged(SeekBar b,int value,boolean fromUser){
                if(RUST_AVAILABLE && fromUser)nativeSetAxis(0,(value-100)*327);
                refreshPad();
            }
            @Override public void onStartTrackingTouch(SeekBar b){}
            @Override public void onStopTrackingTouch(SeekBar b){}
        });
        Button reset=outlined("Centralizar analógico");add(p,reset,14);
        reset.setOnClickListener(v->{axis.setProgress(100);if(RUST_AVAILABLE)nativeSetAxis(0,0);refreshPad();});
        LinearLayout haptics=panel("VIBRAÇÃO / HAPTICS","Feedback tátil nos botões virtuais e nas ações da biblioteca. Usa o motor de vibração do Android.");
        add(content,haptics,14);
        Switch toggleVibe=new Switch(this);
        toggleVibe.setText("Ativar vibração");
        toggleVibe.setTextColor(LIGHT);
        toggleVibe.setTextSize(15);
        toggleVibe.setChecked(getPreferences(MODE_PRIVATE).getBoolean("vibration",true));
        add(haptics,toggleVibe,12);
        TextView hapticInfo=text(Haptics.diagnostic(this),12,MUTED,false);
        add(haptics,hapticInfo,12);
        toggleVibe.setOnCheckedChangeListener((buttonView,isChecked)->{
            getPreferences(MODE_PRIVATE).edit().putBoolean("vibration",isChecked).apply();
            if(isChecked)Haptics.pulse(this,toggleVibe,120,false);
            hapticInfo.setText(Haptics.diagnostic(this));
        });
        TextView intensity=text("Intensidade: "+Haptics.strength(this)+"%",13,LIGHT,false);
        add(haptics,intensity,12);
        SeekBar strength=new SeekBar(this);
        strength.setMax(90);
        strength.setProgress(Math.max(0,Math.min(90,Haptics.strength(this)-10)));
        add(haptics,strength,6);
        strength.setOnSeekBarChangeListener(new SeekBar.OnSeekBarChangeListener(){
            @Override public void onProgressChanged(SeekBar bar,int progress,boolean fromUser){
                if(!fromUser)return;
                int requested=progress+10;
                getPreferences(MODE_PRIVATE).edit().putInt("hapticStrength",requested).apply();
                intensity.setText("Intensidade: "+requested+"%");
            }
            @Override public void onStartTrackingTouch(SeekBar bar){}
            @Override public void onStopTrackingTouch(SeekBar bar){}
        });
        Button testVibe=button("Testar motor de vibração (2 pulsos)");
        add(haptics,testVibe,12);
        testVibe.setOnClickListener(v->{
            String result=Haptics.pulse(this,testVibe,200,true);
            hapticInfo.setText(Haptics.diagnostic(this)+"\n\nTeste: "+result);
        });
        Button refreshHaptic=outlined("Atualizar diagnóstico da vibração");
        add(haptics,refreshHaptic,10);
        refreshHaptic.setOnClickListener(v->hapticInfo.setText(Haptics.diagnostic(this)));
        status("Gamepad físico","Android KeyEvent + MotionEvent reconhecem os botões e eixos; a tradução completa XInput do console ainda não existe.");
    }
    private void refreshPad() {
        if(padStatus!=null && tabIndex==3 && RUST_AVAILABLE)padStatus.setText(nativePadStatus());
    }
    private void press(int mask,boolean pressed) {
        if(pressed)vibrate(15);
        if(!RUST_AVAILABLE)return;
        nativeSetButton(mask,pressed);refreshPad();
    }
    @Override public boolean dispatchKeyEvent(KeyEvent event) {
        if(arcadeActive){
            int key=event.getKeyCode();
            if(key==KeyEvent.KEYCODE_DPAD_UP||key==KeyEvent.KEYCODE_W){
                arcadeUp=event.getAction()==KeyEvent.ACTION_DOWN;
                return true;
            }
            if(key==KeyEvent.KEYCODE_DPAD_DOWN||key==KeyEvent.KEYCODE_S){
                arcadeDown=event.getAction()==KeyEvent.ACTION_DOWN;
                return true;
            }
        }
        int mask=0;
        switch(event.getKeyCode()){
            case KeyEvent.KEYCODE_DPAD_UP:mask=1;break;
            case KeyEvent.KEYCODE_DPAD_LEFT:mask=2;break;
            case KeyEvent.KEYCODE_DPAD_DOWN:mask=4;break;
            case KeyEvent.KEYCODE_DPAD_RIGHT:mask=8;break;
            case KeyEvent.KEYCODE_BUTTON_A:mask=16;break;
            case KeyEvent.KEYCODE_BUTTON_B:mask=32;break;
            case KeyEvent.KEYCODE_BUTTON_X:mask=64;break;
            case KeyEvent.KEYCODE_BUTTON_Y:mask=128;break;
            case KeyEvent.KEYCODE_BUTTON_L1:mask=256;break;
            case KeyEvent.KEYCODE_BUTTON_R1:mask=512;break;
            case KeyEvent.KEYCODE_BUTTON_START:mask=1024;break;
            case KeyEvent.KEYCODE_BUTTON_SELECT:mask=2048;break;
        }
        if(mask!=0&&RUST_AVAILABLE) {
            if(event.getAction()==KeyEvent.ACTION_DOWN && event.getRepeatCount()==0)press(mask,true);
            if(event.getAction()==KeyEvent.ACTION_UP)press(mask,false);
            return true;
        }
        return super.dispatchKeyEvent(event);
    }
    @Override public boolean onGenericMotionEvent(MotionEvent event) {
        if(RUST_AVAILABLE && (event.getSource()&InputDevice.SOURCE_JOYSTICK)==InputDevice.SOURCE_JOYSTICK
            && event.getAction()==MotionEvent.ACTION_MOVE){
            if(arcadeActive){arcadeAxis=event.getAxisValue(MotionEvent.AXIS_Y);}
            nativeSetAxis(0,(int)(event.getAxisValue(MotionEvent.AXIS_X)*32767));
            nativeSetAxis(1,(int)(event.getAxisValue(MotionEvent.AXIS_Y)*32767));
            nativeSetAxis(2,(int)(event.getAxisValue(MotionEvent.AXIS_Z)*32767));
            nativeSetAxis(3,(int)(event.getAxisValue(MotionEvent.AXIS_RZ)*32767));
            nativeSetAxis(4,(int)(event.getAxisValue(MotionEvent.AXIS_LTRIGGER)*255));
            nativeSetAxis(5,(int)(event.getAxisValue(MotionEvent.AXIS_RTRIGGER)*255));
            refreshPad();return true;
        }
        return super.onGenericMotionEvent(event);
    }

    private void addRefreshControls(LinearLayout host){
        LinearLayout p=panel("TELA — 60 / 90 / 120 Hz",
            "Selecione um modo que sua tela realmente ofereça. O Android pode recusar a solicitação. Não altera automaticamente o FPS do Xenia.");
        add(host,p,13);
        TextView screen=text(DisplayTuning.modesText(this),12,MUTED,false);
        add(p,screen,10);
        LinearLayout modes=linear(true);
        String[] labels={"Auto","60 Hz","90 Hz","120 Hz"};
        int[] rates={0,60,90,120};
        for(int i=0;i<rates.length;i++){
            final int hz=rates[i];
            Button btn=outlined(labels[i]);
            btn.setTextSize(11);
            LinearLayout.LayoutParams params=new LinearLayout.LayoutParams(0,dp(48),1f);
            params.setMargins(dp(2),0,dp(2),0);
            modes.addView(btn,params);
            if(!DisplayTuning.supports(this,hz))btn.setEnabled(false);
            btn.setOnClickListener(v->{
                DisplayTuning.save(this,hz);
                screen.setText(DisplayTuning.modesText(this)+"\n"+DisplayTuning.apply(this));
            });
        }
        add(p,modes,10);
        add(p,text("90/120 Hz só aparecem habilitados em telas compatíveis. "
            +"A atividade Xenia/Vulkan controla seu próprio intervalo entre quadros.",12,MUTED,false),9);
    }

    private void showNativeArcade(){
        if(!RUST_AVAILABLE){
            Toast.makeText(this,"Nucleo Rust indisponível",Toast.LENGTH_LONG).show();
            return;
        }
        stopArcade();
        if(surface!=null){surface.onPause();surface=null;}
        content.removeAllViews();
        heading("Nexus Paddle — JOGO NATIVO RUST",
            "Original, executado pelo X360 Nexus sem Xenia. Use os botões ou D-pad/analógico USB ou Bluetooth.");
        add(content,text("Motor: Rust ARM64 • Renderizador próprio 320×180 • Sem ISO Xbox 360",12,GREEN,false),10);
        arcadeImage=new ImageView(this);
        arcadeImage.setScaleType(ImageView.ScaleType.FIT_CENTER);
        arcadeImage.setBackgroundColor(BG);
        arcadeBitmap=Bitmap.createBitmap(320,180,Bitmap.Config.ARGB_8888);
        LinearLayout.LayoutParams lp=new LinearLayout.LayoutParams(-1,dp(235));
        lp.topMargin=dp(14);
        content.addView(arcadeImage,lp);
        arcadeOutput=text("O jogo está iniciando...",12,LIGHT,false);
        add(content,arcadeOutput,10);
        LinearLayout row=linear(true);
        Button up=button("↑ CIMA");
        Button down=button("↓ BAIXO");
        LinearLayout.LayoutParams half=new LinearLayout.LayoutParams(0,dp(68),1f);
        half.setMargins(dp(3),0,dp(3),0);
        row.addView(up,half);
        LinearLayout.LayoutParams halfTwo=new LinearLayout.LayoutParams(0,dp(68),1f);
        halfTwo.setMargins(dp(3),0,dp(3),0);
        row.addView(down,halfTwo);
        up.setOnTouchListener((v,e)->{
            if(e.getActionMasked()==MotionEvent.ACTION_DOWN){
                arcadeUp=true;Haptics.pulse(this,v,40,false);
            }else if(e.getActionMasked()==MotionEvent.ACTION_UP||
                e.getActionMasked()==MotionEvent.ACTION_CANCEL)arcadeUp=false;
            return true;
        });
        down.setOnTouchListener((v,e)->{
            if(e.getActionMasked()==MotionEvent.ACTION_DOWN){
                arcadeDown=true;Haptics.pulse(this,v,40,false);
            }else if(e.getActionMasked()==MotionEvent.ACTION_UP||
                e.getActionMasked()==MotionEvent.ACTION_CANCEL)arcadeDown=false;
            return true;
        });
        add(content,row,12);
        Button reset=outlined("Reiniciar partida");
        add(content,reset,10);
        reset.setOnClickListener(v->{nativeArcadeReset();arcadeLastNanos=0;});
        Button back=outlined("Voltar à biblioteca");
        add(content,back,9);
        back.setOnClickListener(v->showTab(1));
        nativeArcadeReset();
        arcadeActive=true;
        arcadeFrames=0;
        arcadeLastNanos=0;
        arcadeCallback=new Choreographer.FrameCallback(){
            @Override public void doFrame(long frameTimeNanos){
                if(!arcadeActive)return;
                long elapsed=arcadeLastNanos==0?16:
                    Math.max(0,Math.min(50,(frameTimeNanos-arcadeLastNanos)/1000000L));
                arcadeLastNanos=frameTimeNanos;
                int mask=(arcadeUp||arcadeAxis<-0.25f?1:0)|
                    (arcadeDown||arcadeAxis>0.25f?2:0);
                try{
                    int[] frame=nativeArcadeFrame(mask,(int)elapsed);
                    if(frame!=null&&frame.length==320*180){
                        arcadeBitmap.setPixels(frame,0,320,0,0,320,180);
                        arcadeImage.setImageBitmap(arcadeBitmap);
                    }
                    if(++arcadeFrames%30==0)arcadeOutput.setText(nativeArcadeStatus());
                }catch(Throwable err){
                    arcadeActive=false;
                    arcadeOutput.setText("Erro no jogo nativo: "+err.getMessage());
                    return;
                }
                Choreographer.getInstance().postFrameCallback(this);
            }
        };
        Choreographer.getInstance().postFrameCallback(arcadeCallback);
    }
    private void stopArcade(){
        arcadeActive=false;
        arcadeUp=false;
        arcadeDown=false;
        arcadeAxis=0.0f;
        if(arcadeCallback!=null){
            Choreographer.getInstance().removeFrameCallback(arcadeCallback);
            arcadeCallback=null;
        }
        arcadeImage=null;
        arcadeBitmap=null;
        arcadeOutput=null;
    }

    private void executeSelectedPpc(Uri uri){
        new Thread(()->{
            String report;
            try(InputStream in=getContentResolver().openInputStream(uri)){
                if(in==null)throw new IOException("Arquivo não encontrado");
                ByteArrayOutputStream buffer=new ByteArrayOutputStream();
                byte[] chunk=new byte[4096];
                int n;
                while((n=in.read(chunk))!=-1){
                    if(buffer.size()+n>65536)throw new IOException("Programa excede 64 KiB");
                    buffer.write(chunk,0,n);
                }
                if(!RUST_AVAILABLE)throw new IOException("Biblioteca Rust ausente");
                report=nativeExecuteRawPpc(buffer.toByteArray());
            }catch(Throwable err){
                report="Falha no interpretador PowerPC: "+err.getMessage();
            }
            final String msg=report;
            runOnUiThread(()->new android.app.AlertDialog.Builder(this)
                .setTitle("Nexus Core — PowerPC")
                .setMessage(msg)
                .setPositiveButton("OK",null).show());
        },"nexus-raw-ppc").start();
    }

    private void showXboxProfiles(){
        if(!ExternalXboxBackend.isIntegrated(this)){
            Toast.makeText(this,"Perfis reais precisam do Xenia integrado",Toast.LENGTH_LONG).show();
            return;
        }
        new android.app.AlertDialog.Builder(this)
            .setTitle("Perfis Xbox 360 — Xenia integrado")
            .setMessage("Gerencie contas locais offline. O perfil selecionado é usado na próxima inicialização do Minecraft e outros jogos. Não é login Xbox Live.")
            .setItems(new String[]{"Criar perfil local","Listar e selecionar perfil","Abrir interface completa Xenia"},
                (dialog,index)->{
                    if(index==0)createXboxProfile();
                    else if(index==1)listXboxProfiles();
                    else ExternalXboxBackend.openProject(this);
                }).setNegativeButton("Fechar",null).show();
    }
    private void createXboxProfile(){
        EditText gamer=new EditText(this);
        gamer.setSingleLine(true);
        gamer.setHint("Gamertag (1 a 15 caracteres)");
        new android.app.AlertDialog.Builder(this)
            .setTitle("Criar perfil Xbox 360 local")
            .setView(gamer)
            .setNegativeButton("Cancelar",null)
            .setPositiveButton("Criar",(d,which)->{
                String name=gamer.getText().toString();
                new Thread(()->{
                    String result;
                    try{result=XboxProfiles.create(this,name);}
                    catch(Exception err){result="Falha ao criar perfil: "+XboxProfiles.error(err);}
                    final String done=result;
                    runOnUiThread(()->new android.app.AlertDialog.Builder(this)
                        .setTitle("Perfil de jogador")
                        .setMessage(done).setPositiveButton("OK",null).show());
                },"xenia-player-create").start();
            }).show();
    }
    private void listXboxProfiles(){
        new Thread(()->{
            java.util.List<XboxProfiles.Player> list;
            try{list=XboxProfiles.list(this);}
            catch(Exception err){
                String message="Falha ao listar perfis: "+XboxProfiles.error(err);
                runOnUiThread(()->new android.app.AlertDialog.Builder(this)
                    .setTitle("Perfis Xbox 360").setMessage(message)
                    .setPositiveButton("OK",null).show());
                return;
            }
            runOnUiThread(()->{
                if(list.isEmpty()){
                    new android.app.AlertDialog.Builder(this)
                        .setTitle("Nenhum perfil encontrado")
                        .setMessage("Crie seu primeiro perfil local para jogar.")
                        .setPositiveButton("Criar",(d,w)->createXboxProfile())
                        .setNegativeButton("Fechar",null).show();
                    return;
                }
                String[] labels=new String[list.size()];
                for(int i=0;i<list.size();i++)labels[i]=list.get(i).toString();
                new android.app.AlertDialog.Builder(this)
                    .setTitle("Selecionar perfil Xbox 360")
                    .setItems(labels,(d,which)->new Thread(()->{
                        String result;
                        try{result=XboxProfiles.setActive(this,list.get(which).xuid);}
                        catch(Exception err){result="Falha ao selecionar: "+XboxProfiles.error(err);}
                        final String done=result;
                        runOnUiThread(()->new android.app.AlertDialog.Builder(this)
                            .setTitle("Perfil de jogador")
                            .setMessage(done).setPositiveButton("OK",null).show());
                    },"xenia-player-select").start())
                    .setNegativeButton("Fechar",null).show();
            });
        },"xenia-player-list").start();
    }

    private void about() {
        heading("Sobre o X360 Nexus","Build Android 10.4 • Nexus Core + Xenia integrado");
        status("ARQUITETURA","Android SDK + OpenGL ES 2.0 + AudioTrack + Rust JNI aarch64.");
        status("SUPORTE ATUAL","Dois modos: Nexus Core Rust com jogo original e PowerPC experimental, e Xenia/XenDroid ARM64 integrado para jogos Xbox 360; perfis locais e gráficos, modos de tela 60/90/120Hz e otimizações Mali.");
        status("MOTOR XENIA MALI",ExternalXboxBackend.isIntegrated(this)
            ?"Nucleo Xenia ARM64 integrado ao APK. Execucao de jogos ocorre em processo de emulacao proprio (XenDroid), com Vulkan."
            :"Apenas integracao com motor externo. Instale o emulador XenDroid separadamente.");
        status("LIMITAÇÕES IMPORTANTES",
            "Nexus Core sem Xenia não executa ISOs/XEX comerciais: somente jogo original Rust e programas PowerPC experimentais. Xenia integrado permanece disponível para os jogos suportados. 90/120Hz são pedidos à tela do Nexus, não garantia de FPS no Xenia.");
        status("PRIVACIDADE","Arquivos escolhidos permanecem no aparelho. A biblioteca guarda URIs locais, nomes, tamanhos e resultados de inspeção.");
    }
    @Override protected void onActivityResult(int request,int result,Intent data){
        super.onActivityResult(request,result,data);
        if(result!=RESULT_OK||data==null)return;
        if(request==PICK_NATIVE_PPC){
            if(data.getData()!=null)executeSelectedPpc(data.getData());
            return;
        }
        if(request==PICK_COVER||request==PICK_COVERS){
            processCoverPicker(request,data);
            return;
        }
        if(request!=PICK_GAME||data.getData()==null)return;
        Uri uri=data.getData();
        try{
            getContentResolver().takePersistableUriPermission(uri,Intent.FLAG_GRANT_READ_URI_PERMISSION);
        }catch(SecurityException ignored){}
        String name=uri.getLastPathSegment();long size=-1;
        try(Cursor cursor=getContentResolver().query(uri,null,null,null,null)){
            if(cursor!=null&&cursor.moveToFirst()){
                int ni=cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME);
                int si=cursor.getColumnIndex(OpenableColumns.SIZE);
                if(ni>=0&&!cursor.isNull(ni))name=cursor.getString(ni);
                if(si>=0&&!cursor.isNull(si))size=cursor.getLong(si);
            }
        }catch(Exception ignored){}
        final String nameFinal=name==null?"arquivo":name;
        final long sizeFinal=size;
        final String key=uri.toString();
        JSONArray all=loadLibrary(),updated=new JSONArray();
        for(int i=0;i<all.length();i++){
            JSONObject old=all.optJSONObject(i);
            if(old!=null&&!key.equals(old.optString("uri")))updated.put(old);
        }
        try {
            JSONObject item=new JSONObject();
            item.put("name",nameFinal);item.put("size",sizeFinal);
            item.put("uri",key);item.put("scan","Inspecionando arquivo...");
            updated.put(item);
        }catch(Exception ignored){}
        saveLibrary(updated);showTab(1);
        new Thread(()->inspectFile(uri,nameFinal,key),"x360-file-inspector").start();
    }
    private void inspectFile(Uri uri,String name,String key) {
        String result;
        try {
            if(!RUST_AVAILABLE)result="Biblioteca Rust não disponível";
            else if(name.toLowerCase(Locale.ROOT).endsWith(".iso")
                 || name.toLowerCase(Locale.ROOT).endsWith(".xiso")){
                try(ParcelFileDescriptor original=getContentResolver().openFileDescriptor(uri,"r")){
                    if(original==null)result="Arquivo não pôde ser aberto";
                    else {
                        ParcelFileDescriptor duplicate=ParcelFileDescriptor.dup(original.getFileDescriptor());
                        int raw=duplicate.detachFd(); // ownership transferred to Rust and closed there
                        result=nativeInspectIso(raw);
                    }
                }
            } else {
                byte[] h=new byte[16384];
                int n=0;
                try(InputStream input=getContentResolver().openInputStream(uri)){
                    if(input!=null){
                        while(n<h.length){
                            int read=input.read(h,n,h.length-n);
                            if(read<=0)break;
                            n+=read;
                        }
                    }
                }
                if(n>=4 && h[0]=='X'&&h[1]=='E'&&h[2]=='X'&&h[3]=='2')
                    result=nativeInspectXex(Arrays.copyOf(h,n));
                else result=nativeInspectPackage(Arrays.copyOf(h,n));
            }
        }catch(Throwable e){result="Falha ao inspecionar: "+e.getMessage();}
        final String finalResult=result;
        final String extractedTitleId=titleIdFromScan(result);
        runOnUiThread(()->{
            JSONArray lib=loadLibrary();
            for(int i=0;i<lib.length();i++){
                JSONObject item=lib.optJSONObject(i);
                if(item!=null&&key.equals(item.optString("uri"))){
                    try{
                        item.put("scan",finalResult);
                        if(!extractedTitleId.isEmpty())item.put("titleId",extractedTitleId);
                    }catch(Exception ignored){}
                }
            }
            saveLibrary(lib);
            if(tabIndex==1)showTab(1);
        });
    }
    private void startSound(TextView status, Button toggle){
        synchronized(audioGuard) {
            if(audioPlaying || !RUST_AVAILABLE)return;
            audioPlaying=true;
        }
        audioThread=new Thread(()->{
            AudioTrack track=null;
            String error=null;
            try {
                int rate=48000;
                int min=AudioTrack.getMinBufferSize(rate,
                    AudioFormat.CHANNEL_OUT_MONO,AudioFormat.ENCODING_PCM_16BIT);
                if(min<=0)throw new IllegalStateException("PCM mono 48 kHz indisponível");
                track=new AudioTrack(AudioManager.STREAM_MUSIC,rate,
                    AudioFormat.CHANNEL_OUT_MONO,AudioFormat.ENCODING_PCM_16BIT,
                    Math.max(min,4096),AudioTrack.MODE_STREAM);
                if(track.getState()!=AudioTrack.STATE_INITIALIZED)
                    throw new IllegalStateException("Falha ao inicializar AudioTrack");
                synchronized(audioGuard){audioTrack=track;}
                track.play();
                runOnUiThread(()->{
                    if(tabIndex==2)status.setText("Reproduzindo sinal PCM do Rust...");
                });
                short[] samples=new short[512];
                while(audioPlaying){
                    int count=nativeGenerateAudio(samples,toneFrequency,rate);
                    if(count<=0)throw new IllegalStateException("Erro no sintetizador Rust "+count);
                    int written=track.write(samples,0,count);
                    if(written<0)throw new IllegalStateException("AudioTrack.write retornou "+written);
                }
            }catch(Throwable e){error="Erro de áudio: "+String.valueOf(e.getMessage());}
            finally {
                audioPlaying=false;
                if(track!=null) {
                    try{track.pause();track.flush();}catch(Exception ignored){}
                    track.release();
                }
                synchronized(audioGuard){if(audioTrack==track)audioTrack=null;}
                final String result=error;
                runOnUiThread(()->{
                    if(tabIndex==2){
                        toggle.setText("Tocar teste de áudio");
                        status.setText(result==null?"Áudio parado":result);
                    }
                });
            }
        },"x360-rust-pcm");
        audioThread.start();
    }
    private void stopSound(){audioPlaying=false;}
    @Override protected void onPause(){
        stopArcade();
        super.onPause();stopSound();if(surface!=null)surface.onPause();
    }
    @Override protected void onResume(){
        super.onResume();if(surface!=null)surface.onResume();
        DisplayTuning.apply(this);
        // Re-read the OS grant after returning from Settings. No auto-launch.
        if(tabIndex==1 && ExternalXboxBackend.isIntegrated(this) && content!=null){
            content.post(()->{if(tabIndex==1)showTab(1);});
        }
    }
    @Override protected void onDestroy(){
        stopSound();super.onDestroy();
    }
}
