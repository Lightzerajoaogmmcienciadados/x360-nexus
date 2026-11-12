package com.x360nexus;

import android.app.Activity;
import android.content.ActivityNotFoundException;
import android.content.ClipData;
import android.content.Context;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.net.Uri;
import android.os.Build;
import android.widget.Toast;

/**
 * Explicit opt-in bridge to a separately installed XenDroid-family emulator,
 * built from a real Xenia/ARM64 backend. THIS APK DOES NOT contain Xenia.
 *
 * Verified source contract (XenDroid Mali, main/q664, Nov 2026):
 * - package applicationId: xendroid.compose
 * - exported activity: xendroid.compose.EmulatorHostActivity
 * - ACTION: xendroid.intent.action.xendroid
 * - game_uri extra, or ACTION_VIEW data URI
 * Source: https://github.com/DEGROUCHO/xendroid-mali
 * License and attribution remain with the original developers.
 */
public final class ExternalXboxBackend {
    public static final String PACKAGE = "xendroid.compose";
    private static final String ACTIVITY = "xendroid.compose.EmulatorHostActivity";
    private static final String ACTION = "xendroid.intent.action.xendroid";
    public static final String RELEASE_PAGE =
        "https://github.com/DEGROUCHO/xendroid-mali/releases/latest";
    private ExternalXboxBackend(){}

    public static boolean isIntegrated(Context ctx){
        return PACKAGE.equals(ctx.getPackageName());
    }
    public static boolean isInstalled(Context ctx){
        if(isIntegrated(ctx))return true;
        try{
            ctx.getPackageManager().getPackageInfo(PACKAGE,0);
            return true;
        }catch(PackageManager.NameNotFoundException|SecurityException ex){
            return false;
        }
    }

    public static String diagnostic(Context ctx){
        String backend=isIntegrated(ctx)
            ?"Motor Xenia/XenDroid Mali INTEGRADO neste APK"
            :(isInstalled(ctx)?"Motor XenDroid instalado em outro aplicativo":"Motor XenDroid nao instalado");
        boolean arm64=false;
        for(String abi: Build.SUPPORTED_ABIS){
            if("arm64-v8a".equals(abi))arm64=true;
        }
        boolean vulkan=ctx.getPackageManager().hasSystemFeature(
            PackageManager.FEATURE_VULKAN_HARDWARE_VERSION);
        return backend
            +"\nCPU Android ARM64: "+(arm64?"sim":"nao")
            +"\nVulkan no Android: "+(vulkan?"detectado":"nao detectado")
            +"\nGPU Mali: compatibilidade depende do modelo/driver"
            +(isIntegrated(ctx)
                ?"\nMotor Xenia integrado no APK (processo :emu)."
                :"\nMotor Xenia instalado externamente ao X360 Nexus.");
            +"\nNao garante inicializacao nem desempenho de jogos.";
    }

    public static boolean supportsFile(String filename){
        if(filename==null)return false;
        String f=filename.toLowerCase(java.util.Locale.ROOT);
        return f.endsWith(".iso") || f.endsWith(".xiso") ||
            f.endsWith(".xex") || f.endsWith(".zar") ||
            f.endsWith(".stfs") || f.endsWith(".god") ||
            f.endsWith(".xbla");
    }

    /**
     * Forwards a user-selected game's SAF content URI. The actual Xbox 360
     * JIT, HLE, Vulkan command processing and game execution are entirely
     * performed by XenDroid's separate app/process, if installed.
     */
    public static String play(Activity activity,Uri gameUri) {
        return play(activity,gameUri,null,null);
    }

    public static String play(Activity activity,Uri gameUri,
        String gameName,String titleId){
        if(gameUri==null || gameUri.getScheme()==null)
            return "Caminho do jogo invalido";
        if(!isInstalled(activity))
            return "Instale o XenDroid Mali separadamente antes de jogar";
        try{
            Intent launch=new Intent(ACTION);
            launch.setClassName(PACKAGE,ACTIVITY);
            launch.putExtra("game_uri",gameUri.toString());
            if(gameName!=null && !gameName.isEmpty())
                launch.putExtra("game_name",gameName);
            if(MaliPerformance.validTitleId(titleId))
                launch.putExtra("title_id",titleId.toUpperCase(java.util.Locale.ROOT));
            // Carry a real one-time SAF grant for the external process.
            launch.setData(gameUri);
            launch.setClipData(ClipData.newRawUri("Xbox 360 game",gameUri));
            launch.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION);
            activity.startActivity(launch);
            return isIntegrated(activity)
                ?"Jogo enviado ao motor Xenia integrado. Verifique carregamento e FPS."
                :"Jogo enviado ao XenDroid externo. Compatibilidade depende do jogo.";
        }catch(ActivityNotFoundException e){
            return "XenDroid presente, mas a atividade de execucao nao foi encontrada";
        }catch(SecurityException e){
            return "Android bloqueou o compartilhamento da ISO: "+e.getMessage();
        }catch(RuntimeException e){
            return "Falha ao abrir o emulador externo: "+e.getClass().getSimpleName();
        }
    }

    public static void openProject(Activity activity){
        if(isIntegrated(activity)){
            try{
                Intent full=new Intent();
                full.setClassName(PACKAGE,"xendroid.compose.MainActivity");
                activity.startActivity(full);
            }catch(RuntimeException e){
                Toast.makeText(activity,"Falha ao abrir interface Xenia: "+e.getMessage(),
                    Toast.LENGTH_LONG).show();
            }
            return;
        }
        try{
            Intent browser=new Intent(Intent.ACTION_VIEW,Uri.parse(RELEASE_PAGE));
            activity.startActivity(browser);
        }catch(ActivityNotFoundException e){
            Toast.makeText(activity,RELEASE_PAGE,Toast.LENGTH_LONG).show();
        }
    }
}
