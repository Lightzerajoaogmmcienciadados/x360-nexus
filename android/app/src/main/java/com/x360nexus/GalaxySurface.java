package com.x360nexus;

import android.content.Context;
import android.opengl.GLES20;
import android.opengl.GLSurfaceView;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.FloatBuffer;
import javax.microedition.khronos.egl.EGLConfig;
import javax.microedition.khronos.opengles.GL10;

/** Native OpenGL ES 2.0 GPU diagnostic. This is NOT a Xenos GPU command translator. */
public final class GalaxySurface extends GLSurfaceView {
    public GalaxySurface(Context context) {
        super(context);
        setEGLContextClientVersion(2);
        setPreserveEGLContextOnPause(true);
        setRenderer(new Scene());
        setRenderMode(RENDERMODE_CONTINUOUSLY);
    }

    private static final class Scene implements GLSurfaceView.Renderer {
        private static final String VERTEX =
            "attribute vec2 aPos; attribute vec4 aColor; varying vec4 vColor;" +
            "void main(){gl_Position=vec4(aPos,0.0,1.0);vColor=aColor;}";
        private static final String FRAGMENT =
            "precision mediump float;varying vec4 vColor;" +
            "void main(){gl_FragColor=vColor;}";
        private int program, apos, acolor;
        private long start;
        private final FloatBuffer buffer = ByteBuffer.allocateDirect(4096 * 4)
            .order(ByteOrder.nativeOrder()).asFloatBuffer();
        private int vertices;
        private float t;

        private int shader(int type, String source) {
            int shader = GLES20.glCreateShader(type);
            GLES20.glShaderSource(shader,source);
            GLES20.glCompileShader(shader);
            int[] status=new int[1];
            GLES20.glGetShaderiv(shader,GLES20.GL_COMPILE_STATUS,status,0);
            if(status[0]==0) { GLES20.glDeleteShader(shader); throw new IllegalStateException("GLSL compile failed"); }
            return shader;
        }
        @Override public void onSurfaceCreated(GL10 gl,EGLConfig config) {
            int vs=shader(GLES20.GL_VERTEX_SHADER,VERTEX);
            int fs=shader(GLES20.GL_FRAGMENT_SHADER,FRAGMENT);
            program=GLES20.glCreateProgram();
            GLES20.glAttachShader(program,vs);GLES20.glAttachShader(program,fs);
            GLES20.glLinkProgram(program);
            GLES20.glDeleteShader(vs);GLES20.glDeleteShader(fs);
            apos=GLES20.glGetAttribLocation(program,"aPos");
            acolor=GLES20.glGetAttribLocation(program,"aColor");
            GLES20.glClearColor(0.02f,0.06f,0.04f,1f);
            start=System.nanoTime();
        }
        @Override public void onSurfaceChanged(GL10 gl,int width,int height) {
            GLES20.glViewport(0,0,width,height);
        }
        private void point(float x,float y,float r,float g,float b,float a) {
            buffer.put(x).put(y).put(r).put(g).put(b).put(a);
            vertices++;
        }
        private void line(float x1,float y1,float x2,float y2,float bright) {
            point(x1,y1,.12f,.48f*bright,.23f*bright,1f);
            point(x2,y2,.22f,.95f*bright,.38f*bright,1f);
        }
        @Override public void onDrawFrame(GL10 gl) {
            GLES20.glClear(GLES20.GL_COLOR_BUFFER_BIT);
            if(program==0)return;
            t=(System.nanoTime()-start)/1_000_000_000.0f;
            buffer.clear();vertices=0;
            for(int i=0;i<=12;i++) {
                float u=-1f+i/6f;
                line(u,-.98f,u*.17f,.10f,.50f);
            }
            for(int i=0;i<=9;i++) {
                float y=-.98f+i*.11f;
                float k=(y+.98f)/1.1f;
                line(-1+k*.83f,y,1-k*.83f,y,.35f+.65f*k);
            }
            // Abstract glowing diamond/cube wireframe rotates continuously on the GPU.
            float a=t*.55f;
            float s=(float)Math.sin(a),c=(float)Math.cos(a);
            float[][] v={{-.36f,-.32f},{.36f,-.32f},{.36f,.32f},{-.36f,.32f}};
            float[] xs=new float[4],ys=new float[4];
            for(int i=0;i<4;i++){xs[i]=v[i][0]*c-v[i][1]*s;ys[i]=v[i][0]*s+v[i][1]*c+.28f;}
            for(int i=0;i<4;i++){
                int j=(i+1)%4;
                line(xs[i],ys[i],xs[j],ys[j],1f);
                line(xs[i]*.55f,ys[i]*.55f+.15f,xs[j]*.55f,ys[j]*.55f+.15f,.7f);
                line(xs[i],ys[i],xs[i]*.55f,ys[i]*.55f+.15f,.6f);
            }
            for(int i=0;i<4;i++) {
                float w=(float)Math.sin(t+i*1.8f);
                line(-.85f+i*.48f,.73f+w*.025f,-.70f+i*.48f,.73f-w*.025f,.5f);
            }
            buffer.flip();
            GLES20.glUseProgram(program);
            GLES20.glEnableVertexAttribArray(apos);
            GLES20.glEnableVertexAttribArray(acolor);
            buffer.position(0);
            GLES20.glVertexAttribPointer(apos,2,GLES20.GL_FLOAT,false,6*4,buffer);
            buffer.position(2);
            GLES20.glVertexAttribPointer(acolor,4,GLES20.GL_FLOAT,false,6*4,buffer);
            GLES20.glDrawArrays(GLES20.GL_LINES,0,vertices);
            GLES20.glDisableVertexAttribArray(apos);
            GLES20.glDisableVertexAttribArray(acolor);
        }
    }
}
