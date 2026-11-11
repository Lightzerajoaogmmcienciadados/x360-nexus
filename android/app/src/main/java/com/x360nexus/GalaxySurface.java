package com.x360nexus;
import android.content.Context;
import android.opengl.GLSurfaceView;
import android.opengl.GLES20;
import javax.microedition.khronos.egl.EGLConfig;
import javax.microedition.khronos.opengles.GL10;
/** OpenGL ES 2.0 demo only; does not render Xbox Xenos commands. */
public class GalaxySurface extends GLSurfaceView {
    public GalaxySurface(Context context){super(context);setEGLContextClientVersion(2);setRenderer(new Renderer());setRenderMode(RENDERMODE_CONTINUOUSLY);}
    private static class Renderer implements GLSurfaceView.Renderer{
        float phase=0;
        @Override public void onSurfaceCreated(GL10 gl,EGLConfig cfg){GLES20.glClearColor(.01f,.04f,.08f,1);}
        @Override public void onSurfaceChanged(GL10 gl,int w,int h){GLES20.glViewport(0,0,w,h);}
        @Override public void onDrawFrame(GL10 gl){phase+=.012f;float pulse=(float)(.1+.08*Math.sin(phase));GLES20.glClearColor(.01f,pulse,.09f+.04f*(float)Math.cos(phase),1);GLES20.glClear(GLES20.GL_COLOR_BUFFER_BIT);}
    }
}
