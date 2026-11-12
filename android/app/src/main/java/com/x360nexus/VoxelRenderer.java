package com.x360nexus;

/**
 * Independent ORIGINAL ARM64 C++ voxel raycaster. Does not use Xenia or
 * contain/reproduce Minecraft Xbox 360 copyrighted textures or code.
 */
public final class VoxelRenderer {
    private static final boolean AVAILABLE;
    static {
        boolean ok;
        try {System.loadLibrary("nexusvoxel");ok=true;}
        catch(Throwable e){ok=false;}
        AVAILABLE=ok;
    }
    private VoxelRenderer(){}
    private static native int[] nativeRender(byte[] blocks,float px,float py,float pz,
        float yaw,float pitch,int width,int height);
    public static boolean available(){return AVAILABLE;}
    public static int[] render(byte[] blocks,float[] camera,int width,int height){
        if(!AVAILABLE||blocks==null||camera==null||camera.length!=5)return null;
        return nativeRender(blocks,camera[0],camera[1],camera[2],
            camera[3],camera[4],width,height);
    }
}
