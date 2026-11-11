package com.x360nexus;

import android.app.Activity;
import android.os.Bundle;
import android.content.Intent;
import android.content.SharedPreferences;
import android.content.res.Configuration;
import android.graphics.*;
import android.graphics.drawable.Drawable;
import android.net.Uri;
import android.os.ParcelFileDescriptor;
import android.system.Os;
import android.view.*;
import android.widget.*;

public class MainActivity extends Activity {
    private static final int PICK_ISO=100;
    private static final int GREEN=Color.rgb(55,235,105);
    private static final boolean RUST;
    static { boolean ok;try{System.loadLibrary("x360nexus");ok=true;}catch(Throwable t){ok=false;}RUST=ok; }
    private native String nativeSelfTest();
    private native String nativeMemorySelfTest();
    private native String nativeExecutionSelfTest();
    private native String nativeKernelSelfTest();
    private native String nativeRuntimeSelfTest();
    private native String nativeInspectIso(int fd);
    private native String nativeInspectDefaultXex(int fd);
    private Uri selectedIso;
    private native String nativeRunDemo();
    private native void nativeSetButton(int mask,boolean pressed);
    private native void nativeSetAxis(int axis,int value);
    private native String nativePadStatus();
    private LinearLayout root;
    private TextView output;
    private SharedPreferences prefs;
    private int scale=1;
    private boolean showPad=true;

    private TextView label(String value,int size,int color){TextView t=new TextView(this);t.setText(value);t.setTextSize(size);t.setTextColor(color);t.setPadding(12,12,12,12);return t;}
    private Button button(String title,Runnable run){Button b=new Button(this);b.setText(title);b.setAllCaps(false);b.setOnClickListener(v->run.run());return b;}
    private void report(String s){output.setText(s);}
    @Override public void onCreate(Bundle state){super.onCreate(state);prefs=getSharedPreferences("galaxy",MODE_PRIVATE);scale=prefs.getInt("scale",1);showPad=prefs.getBoolean("pad",true);buildUI();}
    private void buildUI(){
        ScrollView scroll=new ScrollView(this);
        scroll.setFillViewport(true);
        root=new LinearLayout(this);root.setOrientation(LinearLayout.VERTICAL);root.setPadding(18,26,18,24);
        getWindow().getDecorView().setSystemUiVisibility(0);
        root.setBackground(new GalaxyBackground());
        root.addView(label("✕ PROJECT XENON",29,GREEN));
        root.addView(label("4.8: XEX RUNTIME  •  Rust / PowerPC\nISO XDVDFS • Controles • OpenGL ES",15,Color.WHITE));
        root.addView(button("💿 SELECIONAR ISO",()->{
            Intent i=new Intent(Intent.ACTION_OPEN_DOCUMENT);i.addCategory(Intent.CATEGORY_OPENABLE);i.setType("*/*");startActivityForResult(i,PICK_ISO);
        }));
        root.addView(button("🔎 ANALISAR CABEÇALHOS XEX2",()->{
            if(selectedIso==null){report("Selecione primeiro a ISO do jogo.");return;}
            if(!RUST){report("Rust indisponivel");return;}
            try(ParcelFileDescriptor pfd=getContentResolver().openFileDescriptor(selectedIso,"r")){
                if(pfd==null){report("Nao foi possivel abrir a ISO");return;}
                int fd=ParcelFileDescriptor.dup(pfd.getFileDescriptor()).detachFd();
                report(nativeInspectDefaultXex(fd));
            }catch(Exception ex){report("Erro ao analisar XEX: "+ex.getMessage());}
        }));
        root.addView(button("▶ EXECUTAR DEMO POWERPC",()->report(RUST?nativeRunDemo():"Biblioteca Rust indisponivel")));
        root.addView(button("⚙ CONFIGURAÇÕES",this::settings));
        root.addView(button("🎮 MOSTRAR / OCULTAR CONTROLES",()->{showPad=!showPad;prefs.edit().putBoolean("pad",showPad).apply();buildUI();}));
        root.addView(button("🧠 TESTAR MEMÓRIA VIRTUAL",()->report(RUST?nativeMemorySelfTest():"Rust indisponivel")));
        root.addView(button("⚡ TESTAR EXECUTION ENGINE",()->report(RUST?nativeExecutionSelfTest():"Rust indisponivel")));
        root.addView(button("🧩 TESTAR KERNEL HOMEBREW",()->report(RUST?nativeKernelSelfTest():"Rust indisponivel")));
        root.addView(button("🔍 TESTAR XEX RUNTIME",()->report(RUST?nativeRuntimeSelfTest():"Rust indisponivel")));
        root.addView(button("🧪 TESTAR NÚCLEO",()->report(RUST?nativeSelfTest():"Rust indisponivel")));
        output=label("Selecione uma ISO para inspecionar arquivos.\nExecucao de jogos comerciais ainda nao implementada.",15,Color.WHITE);
        root.addView(output);
        if(showPad)setupPad();
        root.addView(new GalaxySurface(this),new LinearLayout.LayoutParams(-1,250));
        scroll.addView(root);setContentView(scroll);
    }
    private void settings(){
        String[] items={"Perfil leve (1x)","Perfil equilibrado (2x)","Perfil visual (3x)","Mostrar controles: "+(showPad?"Sim":"Nao")};
        new android.app.AlertDialog.Builder(this).setTitle("Galaxy • Configuracoes").setItems(items,(d,n)->{
            if(n<3){scale=n+1;prefs.edit().putInt("scale",scale).apply();report("Perfil grafico salvo: "+scale+"x (demo OpenGL ES)");}
            else{showPad=!showPad;prefs.edit().putBoolean("pad",showPad).apply();buildUI();}
        }).setPositiveButton("Fechar",null).show();
    }
    private void padButton(LinearLayout row,String name,int bit){
        Button b=new Button(this);b.setText(name);b.setAllCaps(false);row.addView(b,new LinearLayout.LayoutParams(0,72,1));
        b.setOnTouchListener((v,e)->{int a=e.getActionMasked();if(RUST){if(a==MotionEvent.ACTION_DOWN)nativeSetButton(1<<bit,true);if(a==MotionEvent.ACTION_UP||a==MotionEvent.ACTION_CANCEL)nativeSetButton(1<<bit,false);}return true;});
    }
    private void setupPad(){
        root.addView(label("🎮 CONTROLE VIRTUAL",20,GREEN));
        String[][] rows={{"↑","Y","LB","RB"},{"←","↓","→","X"},{"A","B","Start","Back"}};
        int[][] bits={{8,3,4,5},{10,9,11,2},{0,1,6,7}};
        for(int r=0;r<rows.length;r++){LinearLayout line=new LinearLayout(this);for(int c=0;c<4;c++)padButton(line,rows[r][c],bits[r][c]);root.addView(line);}
        root.addView(label("Analogicos (toque e arraste)",14,Color.WHITE));
        LinearLayout sticks=new LinearLayout(this);sticks.addView(new StickView(0),new LinearLayout.LayoutParams(0,170,1));sticks.addView(new StickView(2),new LinearLayout.LayoutParams(0,170,1));root.addView(sticks);
        LinearLayout triggers=new LinearLayout(this);
        for(int axis=4;axis<=5;axis++){final int a=axis;SeekBar bar=new SeekBar(this);bar.setMax(255);bar.setProgress(0);bar.setOnSeekBarChangeListener(new SeekBar.OnSeekBarChangeListener(){public void onProgressChanged(SeekBar b,int value,boolean user){if(RUST)nativeSetAxis(a,value);}public void onStartTrackingTouch(SeekBar b){}public void onStopTrackingTouch(SeekBar b){}});LinearLayout cell=new LinearLayout(this);cell.setOrientation(LinearLayout.VERTICAL);cell.addView(label(axis==4?"LT":"RT",14,GREEN));cell.addView(bar);triggers.addView(cell,new LinearLayout.LayoutParams(0,-2,1));}root.addView(triggers);
        root.addView(button("ESTADO DOS CONTROLES",()->report(RUST?nativePadStatus():"Rust indisponivel")));
    }
    private class StickView extends View {
        final int first;float x=0,y=0;Paint paint=new Paint(3);
        StickView(int axis){super(MainActivity.this);first=axis;}
        @Override protected void onDraw(Canvas c){float cx=getWidth()/2f,cy=getHeight()/2f,r=Math.min(cx,cy)*0.7f;paint.setColor(0xAA204D36);c.drawCircle(cx,cy,r,paint);paint.setColor(GREEN);c.drawCircle(cx+x*r,cy+y*r,r*.3f,paint);}
        @Override public boolean onTouchEvent(MotionEvent e){int a=e.getActionMasked();float cx=getWidth()/2f,cy=getHeight()/2f,r=Math.max(1,Math.min(cx,cy)*.7f);if(a==MotionEvent.ACTION_UP||a==MotionEvent.ACTION_CANCEL){x=0;y=0;}else{x=Math.max(-1,Math.min(1,(e.getX()-cx)/r));y=Math.max(-1,Math.min(1,(e.getY()-cy)/r));}if(RUST){nativeSetAxis(first,(int)(x*32767));nativeSetAxis(first+1,(int)(y*32767));}invalidate();return true;}
    }
    @Override public boolean onKeyDown(int key,KeyEvent e){int bit=mapKey(key);if(bit>=0&&RUST){nativeSetButton(1<<bit,true);return true;}return super.onKeyDown(key,e);}
    @Override public boolean onKeyUp(int key,KeyEvent e){int bit=mapKey(key);if(bit>=0&&RUST){nativeSetButton(1<<bit,false);return true;}return super.onKeyUp(key,e);}
    private int mapKey(int k){switch(k){case KeyEvent.KEYCODE_BUTTON_A:return 0;case KeyEvent.KEYCODE_BUTTON_B:return 1;case KeyEvent.KEYCODE_BUTTON_X:return 2;case KeyEvent.KEYCODE_BUTTON_Y:return 3;case KeyEvent.KEYCODE_BUTTON_L1:return 4;case KeyEvent.KEYCODE_BUTTON_R1:return 5;case KeyEvent.KEYCODE_BUTTON_START:return 6;case KeyEvent.KEYCODE_BUTTON_SELECT:return 7;case KeyEvent.KEYCODE_DPAD_UP:return 8;case KeyEvent.KEYCODE_DPAD_DOWN:return 9;case KeyEvent.KEYCODE_DPAD_LEFT:return 10;case KeyEvent.KEYCODE_DPAD_RIGHT:return 11;default:return -1;}}
    @Override public boolean onGenericMotionEvent(MotionEvent e){if((e.getSource()&InputDevice.SOURCE_JOYSTICK)==InputDevice.SOURCE_JOYSTICK&&e.getAction()==MotionEvent.ACTION_MOVE&&RUST){nativeSetAxis(0,(int)(e.getAxisValue(MotionEvent.AXIS_X)*32767));nativeSetAxis(1,(int)(e.getAxisValue(MotionEvent.AXIS_Y)*32767));nativeSetAxis(2,(int)(e.getAxisValue(MotionEvent.AXIS_Z)*32767));nativeSetAxis(3,(int)(e.getAxisValue(MotionEvent.AXIS_RZ)*32767));nativeSetAxis(4,(int)(e.getAxisValue(MotionEvent.AXIS_LTRIGGER)*255));nativeSetAxis(5,(int)(e.getAxisValue(MotionEvent.AXIS_RTRIGGER)*255));return true;}return super.onGenericMotionEvent(e);}
    @Override protected void onActivityResult(int request,int result,Intent data){super.onActivityResult(request,result,data);if(request!=PICK_ISO||result!=RESULT_OK||data==null||data.getData()==null)return;Uri uri=data.getData();selectedIso=uri;if(!RUST){report("Rust indisponivel");return;}try(ParcelFileDescriptor pfd=getContentResolver().openFileDescriptor(uri,"r")){if(pfd==null){report("Falha ao abrir arquivo");return;}int fd=ParcelFileDescriptor.dup(pfd.getFileDescriptor()).detachFd();report(nativeInspectIso(fd));}catch(Exception ex){report("Erro ISO: "+ex.getMessage());}}
    private static class GalaxyBackground extends Drawable {
        Paint p=new Paint(3);@Override public void draw(Canvas c){android.graphics.Rect b=getBounds();p.setShader(new LinearGradient(0,0,b.width(),b.height(),new int[]{0xFF030811,0xFF102B2D,0xFF190B35},null,Shader.TileMode.CLAMP));c.drawRect(b,p);p.setShader(null);java.util.Random rand=new java.util.Random(41);for(int i=0;i<180;i++){p.setColor(i%5==0?0xAA50FFAA:0x66FFFFFF);c.drawCircle(rand.nextInt(Math.max(1,b.width())),rand.nextInt(Math.max(1,b.height())),1+rand.nextInt(3),p);}}@Override public void setAlpha(int a){}@Override public void setColorFilter(android.graphics.ColorFilter f){}@Override public int getOpacity(){return android.graphics.PixelFormat.OPAQUE;}
    }
}
