package com.x360nexus;

import android.content.Context;

import java.io.File;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.StandardCopyOption;
import java.util.ArrayList;
import java.util.HashSet;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

/**
 * Safe per-title Xenia Mali profile.
 *
 * XenDroid reads:
 * <externalFilesDir("compose")>/config/<TITLE_ID>.config.toml
 * The path and settings were verified against the upstream XenDroid
 * ConfigStore / PerGamePresets sources. User game files are not changed.
 */
public final class MaliPerformance {
    private static final Pattern VALID_ID=Pattern.compile("(?i)[0-9a-f]{8}");
    private static final Pattern SECTION=Pattern.compile("^\\s*\\[([A-Za-z][A-Za-z0-9]*)\\]\\s*(?:#.*)?$");
    private static final Pattern KEY=Pattern.compile("^\\s*([A-Za-z][A-Za-z0-9_]*)\\s*=.*$");
    private static final int MAX_CONFIG=1024*1024;

    private MaliPerformance(){}

    public static boolean validTitleId(String value) {
        return value!=null && VALID_ID.matcher(value).matches()
            && !value.equalsIgnoreCase("00000000");
    }

    private static String id(String value){
        if(!validTitleId(value))
            throw new IllegalArgumentException("Title ID deve conter 8 caracteres hexadecimais");
        return value.toUpperCase(Locale.ROOT);
    }

    private static File configuration(Context context,String titleId)throws IOException{
        if(!ExternalXboxBackend.isIntegrated(context))
            throw new IOException("Motor Xenia nao incorporado neste APK");
        File base=context.getExternalFilesDir("compose");
        if(base==null)throw new IOException("Armazenamento indisponivel");
        File dir=new File(base,"config");
        if(!dir.isDirectory()&&!dir.mkdirs())
            throw new IOException("Falha ao criar diretorio de configuracoes");
        return new File(dir,id(titleId)+".config.toml");
    }

    private static File backup(File file){
        return new File(file.getParentFile(),file.getName()+".nexus-original.bak");
    }
    private static File missing(File file){
        return new File(file.getParentFile(),file.getName()+".nexus-original.missing");
    }

    private static String read(File file)throws IOException{
        if(!file.isFile())return "";
        if(file.length()>MAX_CONFIG)throw new IOException("Configuracao do jogo maior que 1 MB");
        return new String(Files.readAllBytes(file.toPath()),StandardCharsets.UTF_8);
    }

    private static void atomicWrite(File file,String contents)throws IOException{
        File temp=File.createTempFile("nexus-profile-", ".tmp",file.getParentFile());
        try{
            Files.write(temp.toPath(),contents.getBytes(StandardCharsets.UTF_8));
            try {
                Files.move(temp.toPath(),file.toPath(),StandardCopyOption.ATOMIC_MOVE,
                    StandardCopyOption.REPLACE_EXISTING);
            }catch(java.nio.file.AtomicMoveNotSupportedException ex){
                Files.move(temp.toPath(),file.toPath(),StandardCopyOption.REPLACE_EXISTING);
            }
        }finally{
            Files.deleteIfExists(temp.toPath());
        }
    }

    private static LinkedHashMap<String,LinkedHashMap<String,String>> preset(String mode,int customScale,int fpsCap){
        if(fpsCap!=0&&fpsCap!=30&&fpsCap!=60&&fpsCap!=90&&fpsCap!=120)
            throw new IllegalArgumentException("Limite de FPS invalido");
        if(!("performance".equals(mode)||"balanced".equals(mode)
            ||"quality".equals(mode)||"custom".equals(mode)))
            throw new IllegalArgumentException("Perfil desconhecido");
        int scale="performance".equals(mode)?1:
            ("balanced".equals(mode)?2:
             ("quality".equals(mode)?2:Math.max(1,Math.min(3,customScale))));
        boolean effects="quality".equals(mode);

        LinkedHashMap<String,LinkedHashMap<String,String>> all=new LinkedHashMap<>();
        LinkedHashMap<String,String> gpu=new LinkedHashMap<>();
        // Same 1x Mali performance baseline as XenDroid's source preset.
        gpu.put("draw_resolution_scale_x",Integer.toString(scale));
        gpu.put("draw_resolution_scale_y",Integer.toString(scale));
        gpu.put("native_2x_msaa",effects?"true":"false");
        gpu.put("occlusion_query","\"fake\"");
        gpu.put("framerate_limit",Integer.toString(fpsCap));
        gpu.put("guest_display_refresh_cap","true");
        gpu.put("store_shaders","true");
        gpu.put("gpu_stall_spin_us","0");
        gpu.put("vulkan_mid_frame_submission_draws","0");
        gpu.put("clear_memory_page_state","false");
        gpu.put("half_pixel_offset","true");
        if(effects)gpu.put("anisotropic_override","5");
        all.put("GPU",gpu);

        LinkedHashMap<String,String> vulkan=new LinkedHashMap<>();
        vulkan.put("vulkan_in_pass_resolve","true");
        vulkan.put("vulkan_resolve_to_texture_promote","true");
        vulkan.put("vulkan_resolve_to_texture","true");
        vulkan.put("vulkan_resolve_to_texture_serve","true");
        vulkan.put("vulkan_async_skip_draws","true");
        vulkan.put("vulkan_avoid_geometry_shaders","true");
        vulkan.put("vulkan_pipeline_creation_threads","2");
        all.put("Vulkan",vulkan);

        LinkedHashMap<String,String> display=new LinkedHashMap<>();
        display.put("postprocess_antialiasing",effects?"\"fxaa\"":"\"none\"");
        display.put("postprocess_scaling_and_sharpening",effects?"\"cas\"":"\"\"");
        all.put("Display",display);
        LinkedHashMap<String,String> video=new LinkedHashMap<>();
        video.put("guest_swap_pacing","true");
        video.put("guest_spin_park_adaptive","true");
        all.put("Video",video);
        return all;
    }

    private static void appendMissing(StringBuilder out, String section,
        LinkedHashMap<String,LinkedHashMap<String,String>> all,
        HashSet<String> seen){
        LinkedHashMap<String,String> properties=all.get(section);
        if(properties==null)return;
        for(Map.Entry<String,String> entry:properties.entrySet()){
            String full=section+"|"+entry.getKey();
            if(seen.add(full)){
                out.append(entry.getKey()).append(" = ").append(entry.getValue()).append('\n');
            }
        }
    }

    // Patch only known keys; preserve unrelated config, sections and comments.
    static String merge(String source){
        return merge(source,"performance",1,0);
    }
    static String merge(String source,String mode,int customScale){
        return merge(source,mode,customScale,0);
    }
    static String merge(String source,String mode,int customScale,int fpsCap){
        LinkedHashMap<String,LinkedHashMap<String,String>> values=preset(mode,customScale,fpsCap);
        HashSet<String> seen=new HashSet<>();
        HashSet<String> existingSections=new HashSet<>();
        String current=null;
        StringBuilder out=new StringBuilder(source.length()+1024);
        for(String line:source.split("\n",-1)){
            String normalized=line.replace("\r","");
            Matcher head=SECTION.matcher(normalized);
            if(head.matches()){
                if(current!=null)appendMissing(out,current,values,seen);
                current=head.group(1);
                existingSections.add(current);
            }
            Matcher key=KEY.matcher(normalized);
            if(!head.matches() && current!=null && key.matches()
                && values.containsKey(current)
                && values.get(current).containsKey(key.group(1))){
                String full=current+"|"+key.group(1);
                // Remove a duplicate key if present; duplicate TOML keys are invalid.
                if(!seen.add(full))continue;
                out.append(key.group(1)).append(" = ")
                    .append(values.get(current).get(key.group(1))).append('\n');
            } else {
                out.append(normalized).append('\n');
            }
        }
        if(current!=null)appendMissing(out,current,values,seen);
        for(Map.Entry<String,LinkedHashMap<String,String>> group:values.entrySet()){
            if(existingSections.contains(group.getKey()))continue;
            out.append('\n').append('[').append(group.getKey()).append("]\n");
            appendMissing(out,group.getKey(),values,seen);
        }
        return out.toString();
    }

    /** Reusable synthetic assertion for CI independent of device storage. */
    public static boolean selfTest(){
        String input="[GPU]\nuser_custom_key = 7\ndraw_resolution_scale_x = 3\n\n"
            +"[Audio]\nvolume = 0.5\n";
        String result=merge(input);
        String balanced=merge(input,"balanced",2);
        String quality=merge(input,"quality",2);
        String custom=merge(input,"custom",3);
        String cap=merge(input,"performance",1,120);
        return cap.contains("framerate_limit = 120")
            && cap.contains("vulkan_avoid_geometry_shaders = true")
            && balanced.contains("draw_resolution_scale_x = 2")
            && quality.contains("postprocess_antialiasing = \"fxaa\"")
            && custom.contains("draw_resolution_scale_x = 3")
            && result.contains("user_custom_key = 7")
            && result.contains("draw_resolution_scale_x = 1")
            && result.contains("volume = 0.5")
            && result.contains("[Vulkan]")
            && result.contains("postprocess_antialiasing = \"none\"");
    }

    public static synchronized String apply(Context context,String titleId){
        return apply(context,titleId,"performance",1);
    }
    public static synchronized String apply(Context context,String titleId,String preset,int customScale){
        return apply(context,titleId,preset,customScale,0);
    }
    public static synchronized String apply(Context context,String titleId,String preset,int customScale,int fpsCap){
        try{
            if(fpsCap!=0&&fpsCap!=30&&fpsCap!=60&&fpsCap!=90&&fpsCap!=120)
                return "Limite de FPS invalido";
            // Fail before modifying any file on an invalid profile or scale.
            if(!("performance".equals(preset)||"balanced".equals(preset)
                ||"quality".equals(preset)||"custom".equals(preset)))
                return "Perfil desconhecido";
            if(customScale<1||customScale>3)return "Resolucao aceita: 1x, 2x ou 3x";
            File config=configuration(context,titleId);
            File originalBackup=backup(config);
            File absentMarker=missing(config);
            if(!originalBackup.exists() && !absentMarker.exists()){
                if(config.isFile()){
                    // Snapshot once, before first modification.
                    read(config);
                    Files.copy(config.toPath(),originalBackup.toPath());
                }else{
                    if(!absentMarker.createNewFile())
                        throw new IOException("Nao foi possivel criar marcador de recuperacao");
                }
            }
            String output=merge(read(config),preset,customScale,fpsCap);
            if(output.getBytes(StandardCharsets.UTF_8).length>MAX_CONFIG)
                throw new IOException("Resultado maior que 1 MB");
            atomicWrite(config,output);
            context.getSharedPreferences("x360_xenia_profiles",Context.MODE_PRIVATE).edit()
                .putString(id(titleId),preset+"|"+customScale+"|"+fpsCap).apply();
            return "Perfil "+preset+" aplicado ao jogo "+id(titleId)+
                ". O ajuste sera lido na proxima inicializacao do jogo. FPS nao garantido.";
        }catch(Exception ex){
            return "Nao foi possivel aplicar o perfil: "+ex.getMessage();
        }
    }

    public static synchronized String restore(Context context,String titleId){
        try {
            File config=configuration(context,titleId);
            File originalBackup=backup(config);
            File absentMarker=missing(config);
            if(originalBackup.isFile()){
                atomicWrite(config,read(originalBackup));
                Files.delete(originalBackup.toPath());
                Files.deleteIfExists(absentMarker.toPath());
                context.getSharedPreferences("x360_xenia_profiles",Context.MODE_PRIVATE).edit()
                    .remove(id(titleId)).apply();
                return "Configuracao anterior restaurada para "+id(titleId);
            }
            if(absentMarker.isFile()){
                Files.deleteIfExists(config.toPath());
                Files.delete(absentMarker.toPath());
                context.getSharedPreferences("x360_xenia_profiles",Context.MODE_PRIVATE).edit()
                    .remove(id(titleId)).apply();
                return "Configuracao original (sem substituicao) restaurada para "+id(titleId);
            }
            return "Nenhuma configuracao anterior gerenciada pelo Nexus para "+id(titleId);
        }catch(Exception ex){
            return "Falha ao restaurar perfil: "+ex.getMessage();
        }
    }

    public static String currentProfile(Context context,String titleId){
        if(!validTitleId(titleId))return "Nao definido";
        String raw=context.getSharedPreferences("x360_xenia_profiles",Context.MODE_PRIVATE)
            .getString(titleId.toUpperCase(Locale.ROOT),"");
        if(raw.isEmpty())return "Original";
        String[] fields=raw.split("\\|");
        if(fields.length<2)return "Personalizado";
        String frame=(fields.length>2 && !"0".equals(fields[2]))
            ?" • limite "+fields[2]+" FPS":"";
        switch(fields[0]){
            case "performance":return "Desempenho 1x"+frame;
            case "balanced":return "Equilibrado 2x"+frame;
            case "quality":return "Qualidade 2x + FXAA"+frame;
            case "custom":return "Personalizado "+fields[1]+"x"+frame;
            default:return "Personalizado";
        }
    }

    public static synchronized String setFrameLimit(Context context,String titleId,int fps){
        if(!validTitleId(titleId))return "Title ID invalido";
        String v=context.getSharedPreferences("x360_xenia_profiles",Context.MODE_PRIVATE)
            .getString(titleId.toUpperCase(Locale.ROOT),"performance|1|0");
        String[] parts=v.split("\\|");
        String profile=parts.length>0?parts[0]:"performance";
        int scale=1;
        if(parts.length>1)try{scale=Integer.parseInt(parts[1]);}
            catch(NumberFormatException ignored){}
        return apply(context,titleId,profile,scale,fps);
    }

    public static boolean isManaged(Context context,String titleId){
        if(!validTitleId(titleId) || !ExternalXboxBackend.isIntegrated(context))return false;
        try{
            File f=configuration(context,titleId);
            return backup(f).isFile() || missing(f).isFile();
        }catch(IOException ex){return false;}
    }
}
