package com.x360nexus;

import android.content.Context;
import java.io.File;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.util.ArrayList;
import java.util.List;
import java.util.Locale;

/**
 * Actual local Xbox 360 profile management through the INTEGRATED XenDroid
 * JNI engine. Reflection keeps a standalone X360 Nexus Java build compiling
 * without upstream proprietary artifacts.
 *
 * Xenia/XenDroid's native create_profile/list_profiles handle the profile
 * data format; this class DOES NOT fabricate Xbox Live authentication.
 */
public final class XboxProfiles {
    public static final class Player {
        public final String xuid,gamertag;
        public final boolean active;
        public Player(String xuid,String gamertag,boolean active){
            this.xuid=xuid;this.gamertag=gamertag;this.active=active;
        }
        @Override public String toString(){
            return (active?"✓ ":"")+"Perfil: "+gamertag+"\nXUID: "+xuid;
        }
    }
    private XboxProfiles(){}
    private static Object engine(Context context)throws Exception{
        if(!ExternalXboxBackend.isIntegrated(context))
            throw new IllegalStateException("Motor Xenia nao integrado");
        Class<?> runtime=Class.forName("xendroid.compose.core.EmulatorRuntime");
        Object instance=runtime.getField("INSTANCE").get(null);
        runtime.getMethod("ensureLoaded").invoke(instance);
        return Class.forName("xendroid.compose.Emulator").getField("get").get(null);
    }
    private static String root(Context ctx){
        File data=ctx.getExternalFilesDir("compose");
        if(data==null)throw new IllegalStateException("Armazenamento indisponivel");
        File content=new File(data,"content");
        if(!content.isDirectory()&&!content.mkdirs())
            throw new IllegalStateException("Nao foi possivel criar content");
        return content.getAbsolutePath();
    }
    private static Object store(Context ctx)throws Exception{
        Class<?> clazz=Class.forName("xendroid.compose.settings.ConfigStore");
        return clazz.getConstructor(Context.class).newInstance(ctx.getApplicationContext());
    }
    private static String active(Context ctx)throws Exception{
        Object store=store(ctx);
        Object handle=store.getClass().getMethod("openLiveSnapshot").invoke(store);
        try{
            Object value=handle.getClass().getMethod("getString",String.class,String.class)
                .invoke(handle,"Profiles","logged_profile_slot_0_xuid");
            return value==null?"":value.toString();
        }finally{
            handle.getClass().getMethod("closeString").invoke(handle);
        }
    }
    public static List<Player> list(Context context)throws Exception{
        Object emu=engine(context);
        if(emu==null)throw new IllegalStateException("Xenia nao carregado");
        Object result=emu.getClass().getMethod("list_profiles",String.class)
            .invoke(emu,root(context));
        ArrayList<Player> list=new ArrayList<>();
        if(result==null)return list;
        String selected=active(context);
        for(Object item:(Object[])result){
            Class<?> clazz=item.getClass();
            String xuid=(String)clazz.getField("xuid").get(item);
            String tag=(String)clazz.getField("gamertag").get(item);
            if(xuid!=null)list.add(new Player(xuid,tag==null?"":tag,
                xuid.equalsIgnoreCase(selected)));
        }
        return list;
    }
    public static String create(Context context,String name)throws Exception{
        String gamertag=name==null?"":name.trim();
        if(gamertag.length()<1||gamertag.length()>15
            ||!gamertag.matches("[A-Za-z0-9 _-]+"))
            throw new IllegalArgumentException("Nome: 1-15 caracteres, letras/numeros/espacos");
        Object emu=engine(context);
        if(emu==null)throw new IllegalStateException("Xenia nao carregado");
        String xuid=(String)emu.getClass().getMethod("create_profile",
            String.class,String.class,int.class,int.class)
            .invoke(emu,root(context),gamertag,1,103);
        if(xuid==null)throw new IllegalStateException("Nucleo Xenia nao criou o perfil");
        setActive(context,xuid);
        return "Perfil Xbox 360 local criado e ativado:\n"+gamertag+"\nXUID: "+xuid;
    }
    public static String setActive(Context context,String xuid)throws Exception{
        if(xuid==null||!xuid.matches("(?i)[0-9a-f]{16}"))
            throw new IllegalArgumentException("XUID invalido");
        Object store=store(context);
        Object handle=store.getClass().getMethod("openLive").invoke(store);
        boolean saved=false;
        try{
            handle.getClass().getMethod("putString",String.class,String.class,String.class)
                .invoke(handle,"Profiles","logged_profile_slot_0_xuid",xuid.toUpperCase(Locale.ROOT));
            handle.getClass().getMethod("closeFile").invoke(handle);
            saved=true;
        }finally{
            if(!saved){
                try{handle.getClass().getMethod("closeDiscard").invoke(handle);}
                catch(Exception ignored){}
            }
        }
        return "Perfil local selecionado. Reinicie o jogo para aplicar.";
    }
    public static String error(Exception e){
        Throwable why=e;
        while(why.getCause()!=null)why=why.getCause();
        return why.getMessage()==null?why.getClass().getSimpleName():why.getMessage();
    }
}