package com.x360nexus;

import android.app.Activity;
import android.content.ActivityNotFoundException;
import android.content.ClipData;
import android.content.Context;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.net.Uri;
import android.os.Build;
import android.os.Environment;
import android.provider.Settings;
import android.widget.Toast;

/**
 * Calls the in-package XenDroid Mali / Xenia ARM64 host when integrated, or
 * an externally installed compatible backend when using a launcher-only APK.
 * The Xenia runtime and Vulkan graphics translator belong to XenDroid upstream.
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
                :"\nMotor Xenia instalado externamente ao X360 Nexus.")
            +(isIntegrated(ctx)?("\nAcesso a todos os arquivos: "
                +(hasRequiredStorageAccess(ctx)?"AUTORIZADO":"NAO AUTORIZADO")):"")
            +"\nNao garante inicializacao nem desempenho de jogos.";
    }

    /** The integrated XenDroid q664 runtime checks this before native boot. */
    public static boolean hasRequiredStorageAccess(Context context){
        if(!isIntegrated(context))return true;
        return Build.VERSION.SDK_INT>=30 && Environment.isExternalStorageManager();
    }
    /** Open Android's user-controlled special access screen; never auto-grant. */
    public static void requestStorageAccess(Activity activity){
        if(!isIntegrated(activity))return;
        if(Build.VERSION.SDK_INT<30){
            Toast.makeText(activity,"O Xenia integrado necessita Android 11 ou superior",
                Toast.LENGTH_LONG).show();
            return;
        }
        if(hasRequiredStorageAccess(activity)){
            Toast.makeText(activity,"Acesso aos arquivos ja autorizado",Toast.LENGTH_SHORT).show();
            return;
        }
        try{
            Intent settings=new Intent(Settings.ACTION_MANAGE_APP_ALL_FILES_ACCESS_PERMISSION);
            settings.setData(Uri.fromParts("package",activity.getPackageName(),null));
            activity.startActivity(settings);
        }catch(ActivityNotFoundException unavailable){
            try{
                activity.startActivity(new Intent(Settings.ACTION_MANAGE_ALL_FILES_ACCESS_PERMISSION));
            }catch(ActivityNotFoundException e){
                Toast.makeText(activity,
                    "Configuracoes > Aplicativos > Acesso especial > Acesso a todos os arquivos",
                    Toast.LENGTH_LONG).show();
            }
        }catch(SecurityException denied){
            Toast.makeText(activity,"Nao foi possivel abrir ajustes: "+denied.getMessage(),
                Toast.LENGTH_LONG).show();
        }
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
     * performed by XenDroid's own :emu process WITHIN this APK when integrated.
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
        if(isIntegrated(activity) && !hasRequiredStorageAccess(activity)){
            new android.app.AlertDialog.Builder(activity)
                .setTitle("Permissao necessaria para iniciar o jogo")
                .setMessage("O Xenia integrado precisa de Acesso a todos os arquivos. "
                    +"A ISO pode aparecer na biblioteca, mas o motor Android verifica "
                    +"essa permissao antes de iniciar. Abra os ajustes do celular, "
                    +"conceda a permissao e toque em Iniciar jogo novamente.")
                .setNegativeButton("Cancelar",null)
                .setPositiveButton("Abrir configuracoes",(dialog,button)->
                    requestStorageAccess(activity)).show();
            return "Permissao de arquivos pendente no Android";
        }
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
                ?"Xenia integrado iniciado dentro do X360 Nexus. Aguarde o carregamento."
                :"Jogo encaminhado ao emulador externo. Compatibilidade variavel.";
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
