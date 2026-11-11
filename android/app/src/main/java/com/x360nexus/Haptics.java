package com.x360nexus;

import android.content.Context;
import android.os.Build;
import android.os.VibrationEffect;
import android.os.Vibrator;
import android.os.VibratorManager;
import android.provider.Settings;
import android.view.HapticFeedbackConstants;
import android.view.View;

/**
 * Device haptics diagnostics. Best-effort Android motor + View fallback.
 * A successful vibrator API call does not prove a person felt a vibration.
 */
public final class Haptics {
    private Haptics(){}

    private static Vibrator motor(Context context){
        if(Build.VERSION.SDK_INT>=31) {
            VibratorManager manager=(VibratorManager)
                context.getSystemService(Context.VIBRATOR_MANAGER_SERVICE);
            return manager==null?null:manager.getDefaultVibrator();
        }
        return (Vibrator)context.getSystemService(Context.VIBRATOR_SERVICE);
    }
    public static boolean enabled(Context context){
        return context.getSharedPreferences("MainActivity",Context.MODE_PRIVATE)
            .getBoolean("vibration",true);
    }
    public static int strength(Context context){
        return context.getSharedPreferences("MainActivity",Context.MODE_PRIVATE)
            .getInt("hapticStrength",85);
    }
    /**
     * Prefer a real motor effect, then try native view feedback.
     * User preference and Android device vibration settings are respected.
     */
    public static String pulse(Context context,View view,int millis,boolean test){
        if(!test&&!enabled(context))return "Desativada nos controles";
        try {
            Vibrator vib=motor(context);
            if(vib!=null && vib.hasVibrator()){
                int strength=Math.max(10,Math.min(100,strength(context)));
                int amplitude=Math.max(35,Math.min(255,strength*255/100));
                if(!vib.hasAmplitudeControl())amplitude=VibrationEffect.DEFAULT_AMPLITUDE;
                if(test){
                    long[] times={0,Math.max(150,millis),100,Math.max(150,millis)};
                    int[] amps={0,amplitude,0,amplitude};
                    vib.vibrate(VibrationEffect.createWaveform(times,amps,-1));
                }else{
                    vib.vibrate(VibrationEffect.createOneShot(
                        Math.max(30,Math.min(100,millis)),amplitude));
                }
                return "Sinal solicitado ao motor de vibração do Android";
            }
            if(view!=null && view.performHapticFeedback(
                HapticFeedbackConstants.LONG_PRESS)){
                return "Feedback tátil da interface solicitado";
            }
            return "Motor de vibração indisponível ou bloqueado pelo Android";
        } catch(SecurityException e) {
            return "Permissão VIBRATE não autorizada";
        } catch(RuntimeException e) {
            if(view!=null && view.performHapticFeedback(
                HapticFeedbackConstants.LONG_PRESS))
                return "Fallback: feedback da interface solicitado";
            return "Erro ao solicitar vibração: "+e.getClass().getSimpleName();
        }
    }
    public static String diagnostic(Context context){
        boolean hasMotor=false,amplitude=false;
        try{
            Vibrator vib=motor(context);
            hasMotor=vib!=null&&vib.hasVibrator();
            amplitude=hasMotor&&vib.hasAmplitudeControl();
        }catch(RuntimeException ignored){}
        int system=Settings.System.getInt(context.getContentResolver(),
                Settings.System.HAPTIC_FEEDBACK_ENABLED,1);
        return "Motor presente: "+(hasMotor?"sim":"não")
            +"\nControle de intensidade: "+(amplitude?"sim":"não")
            +"\nFeedback tátil do sistema: "+(system!=0?"habilitado":"desativado")
            +"\nVibração no X360 Nexus: "+(enabled(context)?"ligada":"desligada")
            +"\nIntensidade solicitada: "+strength(context)+"%"
            +"\nAjustes do Samsung: Configurações > Sons e vibração > Intensidade da vibração"
            +"\nO teste solicita vibração, mas não confirma a resposta física.";
    }
}
