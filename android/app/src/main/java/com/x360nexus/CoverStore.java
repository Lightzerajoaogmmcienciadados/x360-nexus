package com.x360nexus;

import android.content.Context;
import android.graphics.Bitmap;
import android.graphics.BitmapFactory;
import android.net.Uri;
import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.security.MessageDigest;
import java.text.Normalizer;
import java.util.Locale;

/**
 * Offline cover cache. Supports any Xbox 360 game title supplied by the user.
 * Images are never downloaded, uploaded or bundled; they are supplied by the
 * Android document picker. Aliases: game URI, normalized title or Title ID.
 */
public final class CoverStore {
    private CoverStore() {}

    public static String normalizedTitle(String filename) {
        if(filename == null) return "";
        String value=filename.replaceFirst("(?i)\\.(iso|xiso|xex|god|stfs|xbla|jpg|jpeg|png|webp)$","");
        value=value.replaceAll("(?i)\\((world|usa|europe|japan|eur|pal|ntsc|region free|pt-br|multi[0-9]*)\\)","");
        value=Normalizer.normalize(value,Normalizer.Form.NFD);
        value=value.replaceAll("\\p{M}+","");
        return value.toLowerCase(Locale.ROOT).replaceAll("[^a-z0-9]","");
    }
    public static String imageAlias(String fileName) {
        String stem=fileName == null?"":fileName.replaceFirst("(?i)\\.(jpg|jpeg|png|webp)$","");
        if(stem.matches("(?i)[0-9a-f]{8}"))return "title:"+stem.toUpperCase(Locale.ROOT);
        return "name:"+normalizedTitle(stem);
    }
    public static boolean supportedName(String name) {
        return name!=null&&name.toLowerCase(Locale.ROOT).matches(".*\\.(jpg|jpeg|png|webp)$");
    }
    public static String gameKey(String gameUri) {return "game:"+gameUri;}
    public static String nameKey(String gameName) {return "name:"+normalizedTitle(gameName);}
    public static String titleKey(String titleId) {return "title:"+titleId.toUpperCase(Locale.ROOT);}
    private static String digest(String key){
        try {
            byte[] d=MessageDigest.getInstance("SHA-256").digest(key.getBytes("UTF-8"));
            StringBuilder text=new StringBuilder();
            for(int i=0;i<20;i++)text.append(String.format(Locale.ROOT,"%02x",d[i]&255));
            return text.toString();
        } catch(Exception e){throw new IllegalStateException("SHA-256 unavailable",e);}
    }
    private static File directory(Context c) throws IOException {
        File path=new File(c.getFilesDir(),"covers");
        if(!path.isDirectory()&&!path.mkdirs())throw new IOException("Cannot create cover cache directory");
        return path;
    }
    public static File file(Context c,String key){
        return new File(new File(c.getFilesDir(),"covers"),digest(key)+".jpg");
    }
    public static File find(Context c,String gameUri,String gameName,String titleId){
        File f=file(c,gameKey(gameUri));
        if(f.isFile())return f;
        if(titleId!=null&&titleId.matches("(?i)[0-9a-f]{8}")){
            f=file(c,titleKey(titleId));
            if(f.isFile())return f;
        }
        f=file(c,nameKey(gameName));
        return f.isFile()?f:null;
    }
    private static Bitmap decode(Context c,Uri uri)throws IOException {
        BitmapFactory.Options bounds=new BitmapFactory.Options();
        bounds.inJustDecodeBounds=true;
        try(InputStream in=c.getContentResolver().openInputStream(uri)){
            if(in==null)throw new IOException("Unable to open cover file");
            BitmapFactory.decodeStream(in,null,bounds);
        }
        int w=bounds.outWidth,h=bounds.outHeight;
        if(w<=0||h<=0||w>20000||h>20000)throw new IOException("Invalid image dimensions");
        BitmapFactory.Options opts=new BitmapFactory.Options();
        opts.inPreferredConfig=Bitmap.Config.RGB_565;
        opts.inSampleSize=1;
        while(w/opts.inSampleSize>480||h/opts.inSampleSize>720)
            opts.inSampleSize*=2;
        try(InputStream in=c.getContentResolver().openInputStream(uri)){
            if(in==null)throw new IOException("Unable to read cover");
            Bitmap bmp=BitmapFactory.decodeStream(in,null,opts);
            if(bmp==null)throw new IOException("Invalid or unsupported image");
            return bmp;
        }
    }
    public static void save(Context context,Uri source,String key)throws IOException {
        Bitmap bitmap=decode(context,source);
        File dir=directory(context);
        File temp=File.createTempFile("cover-", ".tmp",dir);
        try {
            try(FileOutputStream out=new FileOutputStream(temp)){
                if(!bitmap.compress(Bitmap.CompressFormat.JPEG,86,out))
                    throw new IOException("Cover compression failed");
            }
            File target=file(context,key);
            if(target.isFile()&&!target.delete())throw new IOException("Cannot replace cached cover");
            if(!temp.renameTo(target))throw new IOException("Cannot commit cached cover");
        } finally {
            bitmap.recycle();
            if(temp.isFile())temp.delete();
        }
    }
    public static Bitmap preview(File file) {
        if(file==null||!file.isFile())return null;
        BitmapFactory.Options opts=new BitmapFactory.Options();
        opts.inSampleSize=2;
        return BitmapFactory.decodeFile(file.getAbsolutePath(),opts);
    }
    public static void removeGame(Context ctx,String uri) {
        File file=file(ctx,gameKey(uri));
        if(file.isFile())file.delete();
    }
}
