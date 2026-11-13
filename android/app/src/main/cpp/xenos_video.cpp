// EGL / OpenGL ES diagnostic renderer, not guest shader translation.
// GL calls and EGL teardown are confined to a dedicated Android video worker.
#include "xenos_video.hpp"
#include "xenos_pm4.hpp"
#include <jni.h>
#include <android/native_window_jni.h>
#include <android/native_window.h>
#include <EGL/egl.h>
#include <GLES3/gl3.h>
#include <chrono>
#include <algorithm>
#include <cmath>
#include <mutex>
#include <sstream>
#include <string>

namespace {
using Clock=std::chrono::steady_clock;
struct State {
    std::mutex mutex;
    ANativeWindow* window=nullptr;
    EGLDisplay display=EGL_NO_DISPLAY;
    EGLSurface surface=EGL_NO_SURFACE;
    EGLContext context=EGL_NO_CONTEXT;
    GLuint program=0, vbo=0;
    int backend=0, scale=1;
    bool vsync=true;
    unsigned long long frames=0;
    Clock::time_point epoch=Clock::now();
    std::string error="Sem superficie";
} g;
std::string make_error(const char* step){
    std::ostringstream s;s<<step<<": EGL 0x"<<std::hex<<eglGetError();return s.str();
}
void drop_gl() {
    if(g.display!=EGL_NO_DISPLAY){
        if(g.context!=EGL_NO_CONTEXT){
            eglMakeCurrent(g.display,g.surface,g.surface,g.context);
            if(g.vbo) glDeleteBuffers(1,&g.vbo);
            if(g.program) glDeleteProgram(g.program);
            g.vbo=0;g.program=0;
        }
        eglMakeCurrent(g.display,EGL_NO_SURFACE,EGL_NO_SURFACE,EGL_NO_CONTEXT);
        if(g.surface!=EGL_NO_SURFACE)eglDestroySurface(g.display,g.surface);
        if(g.context!=EGL_NO_CONTEXT)eglDestroyContext(g.display,g.context);
        eglTerminate(g.display);
    }
    g.surface=EGL_NO_SURFACE;g.context=EGL_NO_CONTEXT;g.display=EGL_NO_DISPLAY;
}
GLuint shader(GLenum type,const char* src){
    GLuint sh=glCreateShader(type);glShaderSource(sh,1,&src,nullptr);glCompileShader(sh);
    GLint ok=GL_FALSE;glGetShaderiv(sh,GL_COMPILE_STATUS,&ok);
    if(!ok){glDeleteShader(sh);return 0;}return sh;
}
bool setup() {
    if(!g.window){g.error="Sem SurfaceView";return false;}
    if(g.backend==1){g.error="Vulkan solicitado: backend nao implementado (use Auto/GLES)";return false;}
    g.display=eglGetDisplay(EGL_DEFAULT_DISPLAY);
    if(g.display==EGL_NO_DISPLAY || eglInitialize(g.display,nullptr,nullptr)!=EGL_TRUE){g.error=make_error("eglInitialize");drop_gl();return false;}
    static constexpr EGLint attribs[]={EGL_RENDERABLE_TYPE,0x40/*ES3*/,EGL_SURFACE_TYPE,EGL_WINDOW_BIT,EGL_RED_SIZE,8,EGL_GREEN_SIZE,8,EGL_BLUE_SIZE,8,EGL_ALPHA_SIZE,8,EGL_NONE};
    EGLConfig config=nullptr;EGLint count=0;
    if(!eglChooseConfig(g.display,attribs,&config,1,&count) || count==0){g.error="EGL: contexto OpenGL ES 3 indisponivel";drop_gl();return false;}
    constexpr EGLint context_args[]={EGL_CONTEXT_CLIENT_VERSION,3,EGL_NONE};
    g.context=eglCreateContext(g.display,config,EGL_NO_CONTEXT,context_args);
    if(g.context==EGL_NO_CONTEXT){g.error=make_error("eglCreateContext");drop_gl();return false;}
    g.surface=eglCreateWindowSurface(g.display,config,g.window,nullptr);
    if(g.surface==EGL_NO_SURFACE || !eglMakeCurrent(g.display,g.surface,g.surface,g.context)){g.error=make_error("eglMakeCurrent");drop_gl();return false;}
    eglSwapInterval(g.display,g.vsync?1:0);
    static const char* vert="#version 300 es\nlayout(location=0) in vec2 position;\nvoid main(){gl_Position=vec4(position,0.0,1.0);}\n";
    static const char* frag="#version 300 es\nprecision mediump float;out vec4 color;\nvoid main(){color=vec4(0.08,0.80,0.36,1.0);}\n";
    GLuint vs=shader(GL_VERTEX_SHADER,vert),fs=shader(GL_FRAGMENT_SHADER,frag);
    if(!vs||!fs){if(vs)glDeleteShader(vs);if(fs)glDeleteShader(fs);g.error="GLSL demo shaders nao compilados";drop_gl();return false;}
    g.program=glCreateProgram();glAttachShader(g.program,vs);glAttachShader(g.program,fs);
    glLinkProgram(g.program);glDeleteShader(vs);glDeleteShader(fs);
    GLint linked=GL_FALSE;glGetProgramiv(g.program,GL_LINK_STATUS,&linked);
    if(!linked){g.error="GLSL demo link failed";drop_gl();return false;}
    constexpr GLfloat vertices[]={-0.7f,-0.65f, 0.7f,-0.65f, 0.0f,0.70f};
    glGenBuffers(1,&g.vbo);glBindBuffer(GL_ARRAY_BUFFER,g.vbo);
    glBufferData(GL_ARRAY_BUFFER,sizeof(vertices),vertices,GL_STATIC_DRAW);
    g.error="OpenGL ES: diagnostico ativo (triangulo de teste, nao Xenos)";
    return true;
}
std::string draw() {
    if(!g.window) return "Sem janela Android";
    if(g.backend==1) return "Vulkan selecionado, mas nao implementado; nenhuma imagem sintetizada";
    if(g.context==EGL_NO_CONTEXT && !setup())return g.error;
    if(!eglMakeCurrent(g.display,g.surface,g.surface,g.context)){g.error=make_error("eglMakeCurrent frame");drop_gl();return g.error;}
    EGLint width=0,height=0;
    eglQuerySurface(g.display,g.surface,EGL_WIDTH,&width);
    eglQuerySurface(g.display,g.surface,EGL_HEIGHT,&height);
    if(width<=0||height<=0)return "SurfaceView com tamanho zero";
    const float ratio=static_cast<float>(g.scale==0?0.5:g.scale==1?1.0:g.scale==2?1.5:2.0);
    const int rw=std::max(1,int(width*ratio)),rh=std::max(1,int(height*ratio));
    // The surface stays at native resolution. Scale influences the GL viewport
    // only for this demonstration; no Xenos render-target scaling occurs.
    glViewport(0,0,std::min(rw,int(width)),std::min(rh,int(height)));
    const float t=float(g.frames%180)/180.f;
    // A diagnostic SET_CONSTANT2 to app-defined register 0xC000 controls
    // the clear color. This is NOT a translation of actual Xenos EDRAM clears.
    uint32_t color=0;
    if(x360::gpu::processor().diagnostic_clear_color(color)){
        glClearColor(float((color>>24)&255)/255.f,float((color>>16)&255)/255.f,
                     float((color>>8)&255)/255.f,float(color&255)/255.f);
    }else{
        glClearColor(0.025f,0.065f+0.04f*t,0.04f,1.f);
    }
    glClear(GL_COLOR_BUFFER_BIT);
    glUseProgram(g.program);glBindBuffer(GL_ARRAY_BUFFER,g.vbo);
    glEnableVertexAttribArray(0);glVertexAttribPointer(0,2,GL_FLOAT,GL_FALSE,0,nullptr);
    glDrawArrays(GL_TRIANGLES,0,3);
    glDisableVertexAttribArray(0);
    if(!eglSwapBuffers(g.display,g.surface)){g.error=make_error("eglSwapBuffers");drop_gl();return g.error;}
    ++g.frames;
    return "Frame GL diagnosticado";
}
}
namespace x360::gpu {
void set_video_options(int backend,int scale,bool vsync){
    std::lock_guard<std::mutex> lock(g.mutex);
    bool changed = g.backend!=backend || g.vsync!=vsync;
    g.backend=backend;g.scale=scale;g.vsync=vsync;
    // Recreate EGL context next frame on same video worker, not the UI thread.
    if(changed)g.error="Configuracoes aguardando proximo frame";
}
std::string video_status(){
    std::lock_guard<std::mutex> lock(g.mutex);
    const double seconds=std::max(0.001,std::chrono::duration<double>(Clock::now()-g.epoch).count());
    std::ostringstream out;out<<g.error<<"; frames="<<g.frames<<"; media FPS="<<int(g.frames/seconds);
    auto pm4=processor().stats();
    out<<"; PM4="<<pm4.packets<<"; LOAD_ALU="<<pm4.alu_loads<<"; events="<<pm4.event_writes<<"; draws sem shader Xenos="<<pm4.draws_seen;
    if(!pm4.last_event.empty())out<<"; ultimo PM4="<<pm4.last_event;
    return out.str();
}
}
extern "C" JNIEXPORT jstring JNICALL
Java_com_x360nexus_MainActivity_nativeSetVideoSurface(JNIEnv* env,jobject,jobject surface){
    std::lock_guard<std::mutex> lock(g.mutex);
    drop_gl();
    if(g.window){ANativeWindow_release(g.window);g.window=nullptr;}
    if(surface)g.window=ANativeWindow_fromSurface(env,surface);
    g.epoch=Clock::now();g.frames=0;g.error=g.window?"SurfaceView anexada":"SurfaceView liberada";
    return env->NewStringUTF(g.error.c_str());
}
extern "C" JNIEXPORT jstring JNICALL
Java_com_x360nexus_MainActivity_nativeRenderVideoFrame(JNIEnv* env,jobject){
    std::lock_guard<std::mutex> lock(g.mutex);
    auto status=draw();return env->NewStringUTF(status.c_str());
}
extern "C" JNIEXPORT jstring JNICALL
Java_com_x360nexus_MainActivity_nativeVideoStats(JNIEnv* env,jobject){
    auto status=x360::gpu::video_status();return env->NewStringUTF(status.c_str());
}