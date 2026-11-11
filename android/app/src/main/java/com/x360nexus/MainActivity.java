package com.x360nexus;

import android.app.Activity;
import android.os.Bundle;
import android.os.ParcelFileDescriptor;
import android.content.Intent;
import android.content.Context;
import android.database.Cursor;
import android.graphics.Color;
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
import java.util.Locale;

/**
 * X360 Nexus native Android shell. No HTML, browser, JavaScript or WebView.
 * Experimental PowerPC diagnostics, GLES2, PCM output, Android gamepad,
 * file/library inspection. NOT capable of running retail Xbox 360 games.
 */
public final class MainActivity extends Activity {
    private static final int PICK_GAME = 360;
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
    private native String nativeRunDemo();
    private native String nativeInspectIso(int fileDescriptor);
    private native String nativeInspectXex(byte[] bytes);
    private native String nativeExecuteRawPpc(byte[] bytes);
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
        Button go=outlined("Abrir biblioteca de jogos");add(content,go,18);
        go.setOnClickListener(v->showTab(1));
    }
    private void core() {
        heading("Núcleo Rust / Xenon","Ferramentas executadas localmente no aparelho.");
        status("CPU PowerPC experimental",
            "Instruções: addi/addis, mulli, subfic, OR/AND/XOR, comparações, load/store, branches, LR/CTR e extensões.\nAinda faltam MMU, VMX128, SMT, JIT e grande parte da ISA.");
        status("Kernel e memória","Memória big-endian, regiões virtuais de 4 KiB com permissões R/W/X e threads experimentais. Não é uma MMU Xbox 360.");
        status("GPU","Framebuffer RGBA em Rust + cena OpenGL ES 2.0 no Android. Sem tradução de shaders ou comandos Xenos.");
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
    private void library() {
        heading("Biblioteca local","ISO: leitura XDVDFS e análise do default.xex. PPC bruto: execução isolada de diagnóstico. Jogos Xbox 360 ainda indisponíveis.");
        LinearLayout pick=panel("IMPORTAR ISO / XEX / STFS / GOD / XBLA",
            "Armazena URI e metadados localmente. Não envia o arquivo à internet.");
        add(content,pick,15);
        Button b=button("+ Adicionar arquivo");add(pick,b,14);
        b.setOnClickListener(v->{
            Intent open=new Intent(Intent.ACTION_OPEN_DOCUMENT);
            open.addCategory(Intent.CATEGORY_OPENABLE);open.setType("*/*");
            open.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION|Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION);
            startActivityForResult(open,PICK_GAME);
        });
        JSONArray all=loadLibrary();
        add(content,text("Arquivos adicionados: "+all.length(),18,LIGHT,true),22);
        if(all.length()==0){
            LinearLayout empty=panel("Biblioteca vazia","Toque em + Adicionar arquivo para inspecionar seus backups.");
            add(content,empty,12);
        }
        for(int i=0;i<all.length();i++) {
            final JSONObject item=all.optJSONObject(i);
            if(item==null)continue;
            String name=item.optString("name","Arquivo");
            String uri=item.optString("uri","");
            String scan=item.optString("scan","Não verificado");
            long bytes=item.optLong("size",-1);
            LinearLayout card=panel(name,formatBytes(bytes)+"\n"+scan);
            add(content,card,12);
            Button remove=outlined("Remover da biblioteca");
            add(card,remove,10);
            if(name.toLowerCase(Locale.ROOT).endsWith(".ppcbin")
                || name.toLowerCase(Locale.ROOT).endsWith(".nxpp")
                || name.toLowerCase(Locale.ROOT).endsWith(".ppc")) {
                Button execute=button("Executar código PowerPC experimental");
                add(card,execute,10);
                execute.setOnClickListener(v->runRawPpc(Uri.parse(uri),card));
            }
            remove.setOnClickListener(v->{
                JSONArray arr=loadLibrary(),next=new JSONArray();
                for(int k=0;k<arr.length();k++){
                    JSONObject o=arr.optJSONObject(k);
                    if(o!=null&&!uri.equals(o.optString("uri")))next.put(o);
                }
                saveLibrary(next);showTab(1);
            });
        }
        LinearLayout note=panel("EXECUÇÃO DE JOGOS",
            "Agora o carregador também lê o cabeçalho de default.xex dentro das ISOs XDVDFS. Isso ainda não implementa decriptação, mapeamento ou inicialização de jogos comerciais.");
        add(content,note,14);
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
        status("Gamepad físico","Android KeyEvent + MotionEvent reconhecem os botões e eixos; a tradução completa XInput do console ainda não existe.");
    }
    private void refreshPad() {
        if(padStatus!=null && tabIndex==3 && RUST_AVAILABLE)padStatus.setText(nativePadStatus());
    }
    private void press(int mask,boolean pressed) {
        if(!RUST_AVAILABLE)return;
        nativeSetButton(mask,pressed);refreshPad();
    }
    @Override public boolean dispatchKeyEvent(KeyEvent event) {
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
    private void about() {
        heading("Sobre o X360 Nexus","Build Android 6.2 • Experimental nativo");
        status("ARQUITETURA","Android SDK + OpenGL ES 2.0 + AudioTrack + Rust JNI aarch64.");
        status("SUPORTE ATUAL","ISO/XDVDFS e default.xex, tabela opcional XEX2, memória virtual experimental R/W/X, mais operações PPC, biblioteca local, controles Android, GPU diagnóstica e áudio PCM com frequência ajustável.");
        status("LIMITAÇÕES IMPORTANTES","Sem JIT PPC, MMU Xbox completa, APIs XAM/Xbox Kernel, decodificador XMA, tradutor Xenos, renderização 360 e execução de jogos comerciais.");
        status("PRIVACIDADE","Arquivos escolhidos permanecem no aparelho. A biblioteca guarda URIs locais, nomes, tamanhos e resultados de inspeção.");
    }
    @Override protected void onActivityResult(int request,int result,Intent data){
        super.onActivityResult(request,result,data);
        if(request!=PICK_GAME || result!=RESULT_OK||data==null||data.getData()==null)return;
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
                else result="Arquivo catalogado: sem execução. Formato não identificado como XEX2.";
            }
        }catch(Throwable e){result="Falha ao inspecionar: "+e.getMessage();}
        final String finalResult=result;
        runOnUiThread(()->{
            JSONArray lib=loadLibrary();
            for(int i=0;i<lib.length();i++){
                JSONObject item=lib.optJSONObject(i);
                if(item!=null&&key.equals(item.optString("uri"))){
                    try{item.put("scan",finalResult);}catch(Exception ignored){}
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
        super.onPause();stopSound();if(surface!=null)surface.onPause();
    }
    @Override protected void onResume(){
        super.onResume();if(surface!=null)surface.onResume();
    }
    @Override protected void onDestroy(){
        stopSound();super.onDestroy();
    }
}
