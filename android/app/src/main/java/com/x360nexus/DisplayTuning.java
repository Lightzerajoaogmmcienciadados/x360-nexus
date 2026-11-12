package com.x360nexus;

import android.app.Activity;
import android.content.Context;
import android.view.Display;
import android.view.WindowManager;
import java.util.ArrayList;
import java.util.Collections;
import java.util.Locale;

/**
 * Request (not force) the actual screen mode supported by this device.
 * This only applies to the X360 Nexus Activity and its original Rust
 * arcade. The separately managed Xenia emulator-host Activity controls
 * its own Surface/Vulkan frame pacing, and isn't overridden by this class.
 */
public final class DisplayTuning {
    public static final int[] MODES={0,60,90,120};
    private DisplayTuning(){}
    public static int selected(Context ctx){
        return ctx.getSharedPreferences("x360_nexus_display",Context.MODE_PRIVATE)
            .getInt("refresh",0);
    }
    public static void save(Activity activity,int mode){
        if(mode!=0&&mode!=60&&mode!=90&&mode!=120)return;
        activity.getSharedPreferences("x360_nexus_display",Context.MODE_PRIVATE)
            .edit().putInt("refresh",mode).apply();
        apply(activity);
    }
    private static Display get(Activity a){
        return a.getWindowManager().getDefaultDisplay();
    }
    public static boolean supports(Activity activity,int wanted){
        if(wanted==0)return true;
        Display display=get(activity);
        if(display==null)return false;
        Display.Mode current=display.getMode();
        for(Display.Mode mode:display.getSupportedModes()){
            if(mode.getPhysicalWidth()==current.getPhysicalWidth()
                &&mode.getPhysicalHeight()==current.getPhysicalHeight()
                &&Math.abs(mode.getRefreshRate()-wanted)<1.1f)return true;
        }
        return false;
    }
    public static String modesText(Activity activity){
        Display display=get(activity);
        if(display==null)return "A tela nao informou taxas de atualizacao";
        Display.Mode current=display.getMode();
        ArrayList<String> candidates=new ArrayList<>();
        for(Display.Mode m:display.getSupportedModes()){
            if(m.getPhysicalWidth()==current.getPhysicalWidth()
                &&m.getPhysicalHeight()==current.getPhysicalHeight()){
                String rate=String.format(Locale.ROOT,"%.1f Hz",m.getRefreshRate());
                if(!candidates.contains(rate))candidates.add(rate);
            }
        }
        Collections.sort(candidates);
        return "Modos da tela: "+android.text.TextUtils.join(" • ",candidates)
            +"\nModo atual: "+String.format(Locale.ROOT,"%.1f Hz",current.getRefreshRate())
            +"\nSolicitado Nexus: "+(selected(activity)==0?"Automático":selected(activity)+" Hz")
            +"\nNão libera 90/120 FPS em jogos Xbox 360; o jogo e o Xenia "
            +"definem o ritmo de quadros.";
    }
    public static String apply(Activity activity){
        int wanted=selected(activity);
        if(wanted!=0&&!supports(activity,wanted)){
            return wanted+" Hz indisponivel neste modelo de tela.";
        }
        try{
            WindowManager.LayoutParams params=activity.getWindow().getAttributes();
            if(wanted==0){
                params.preferredDisplayModeId=0;
                params.preferredRefreshRate=0.0f;
            }else{
                Display display=get(activity);
                Display.Mode current=display.getMode();
                for(Display.Mode m:display.getSupportedModes()){
                    if(m.getPhysicalWidth()==current.getPhysicalWidth()
                        &&m.getPhysicalHeight()==current.getPhysicalHeight()
                        &&Math.abs(m.getRefreshRate()-wanted)<1.1f){
                        params.preferredDisplayModeId=m.getModeId();
                        params.preferredRefreshRate=m.getRefreshRate();
                        break;
                    }
                }
            }
            activity.getWindow().setAttributes(params);
            return "Taxa solicitada: "+(wanted==0?"Automático":wanted+" Hz")
                +". Android pode selecionar outro modo.";
        }catch(RuntimeException e){
            return "Ajuste de tela indisponivel: "+e.getClass().getSimpleName();
        }
    }
}