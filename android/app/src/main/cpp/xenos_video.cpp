// Xenos PM4-backed presentation surface. No host-generated diagnostic geometry.
// Real game draw calls require validated translated Xenos microcode and EDRAM;
// until implemented we present a deterministic dark framebuffer, not a fake game.
#include "xenos_video.hpp"
#include "xenos_pm4.hpp"
#include <jni.h>
#include <android/native_window_jni.h>
#include <android/native_window.h>
#include <EGL/egl.h>
#include <EGL/eglext.h>
#include <GLES3/gl3.h>
#include <algorithm>
#include <chrono>
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
    int backend=0,scale=1;
    bool vsync=true;
    bool es32=false;
    unsigned long long frames=0;
    Clock::time_point epoch=Clock::now();
    std::string error="Sem superficie";
} g;
std::string egl_error(const char* operation){
    std::ostringstream out;
    out<<operation<<": EGL 0x"<<std::hex<<eglGetError();
    return out.str();
}
void release_egl(){
    if(g.display!=EGL_NO_DISPLAY){
        eglMakeCurrent(g.display,EGL_NO_SURFACE,EGL_NO_SURFACE,EGL_NO_CONTEXT);
        if(g.surface!=EGL_NO_SURFACE)eglDestroySurface(g.display,g.surface);
        if(g.context!=EGL_NO_CONTEXT)eglDestroyContext(g.display,g.context);
        eglTerminate(g.display);
    }
    g.surface=EGL_NO_SURFACE;g.context=EGL_NO_CONTEXT;g.display=EGL_NO_DISPLAY;
    g.es32=false;
}
bool setup(){
    if(!g.window){g.error="Sem SurfaceView";return false;}
    if(g.backend==1){g.error="Vulkan nao implementado";return false;}
    g.display=eglGetDisplay(EGL_DEFAULT_DISPLAY);
    if(g.display==EGL_NO_DISPLAY||!eglInitialize(g.display,nullptr,nullptr)){
        g.error=egl_error("eglInitialize");release_egl();return false;
    }
    static constexpr EGLint attribs[]{
        EGL_RENDERABLE_TYPE,EGL_OPENGL_ES3_BIT_KHR,
        EGL_SURFACE_TYPE,EGL_WINDOW_BIT,EGL_RED_SIZE,8,
        EGL_GREEN_SIZE,8,EGL_BLUE_SIZE,8,EGL_ALPHA_SIZE,8,EGL_NONE
    };
    EGLConfig config=nullptr;EGLint configurations=0;
    if(!eglChooseConfig(g.display,attribs,&config,1,&configurations)||configurations<1){
        g.error="EGL nao suporta OpenGL ES 3";release_egl();return false;
    }
    // Request ES 3.2 first; retain a GLES3 fallback for older Mali devices.
    static constexpr EGLint es32[]{
        EGL_CONTEXT_MAJOR_VERSION_KHR,3,EGL_CONTEXT_MINOR_VERSION_KHR,2,EGL_NONE
    };
    g.context=eglCreateContext(g.display,config,EGL_NO_CONTEXT,es32);
    g.es32=g.context!=EGL_NO_CONTEXT;
    if(!g.es32){
        static constexpr EGLint es3[]{EGL_CONTEXT_CLIENT_VERSION,3,EGL_NONE};
        g.context=eglCreateContext(g.display,config,EGL_NO_CONTEXT,es3);
    }
    if(g.context==EGL_NO_CONTEXT){g.error=egl_error("eglCreateContext");release_egl();return false;}
    g.surface=eglCreateWindowSurface(g.display,config,g.window,nullptr);
    if(g.surface==EGL_NO_SURFACE||
       !eglMakeCurrent(g.display,g.surface,g.surface,g.context)){
        g.error=egl_error("eglMakeCurrent");release_egl();return false;
    }
    eglSwapInterval(g.display,g.vsync?1:0);
    const char* renderer=reinterpret_cast<const char*>(glGetString(GL_RENDERER));
    const char* version=reinterpret_cast<const char*>(glGetString(GL_VERSION));
    std::ostringstream msg;
    msg<<"EGL "<<(version?version:"ES3")<<" "<<(renderer?renderer:"GPU desconhecida")
       <<"; aguardando comandos PM4 renderizaveis";
    g.error=msg.str();
    return true;
}
std::string render(){
    if(!g.window)return "Sem SurfaceView";
    if(g.backend==1)return "Vulkan nao implementado";
    if(g.context==EGL_NO_CONTEXT&&!setup())return g.error;
    if(!eglMakeCurrent(g.display,g.surface,g.surface,g.context)){
        g.error=egl_error("eglMakeCurrent frame");release_egl();return g.error;
    }
    EGLint w=0,h=0;
    eglQuerySurface(g.display,g.surface,EGL_WIDTH,&w);
    eglQuerySurface(g.display,g.surface,EGL_HEIGHT,&h);
    if(w<=0||h<=0)return "SurfaceView tamanho invalido";
    // Native fullscreen SurfaceView, with no fabricated triangles, sprites,
    // shaders, motion, test colors or placeholder Xenos render targets.
    glViewport(0,0,w,h);
    glClearColor(0.012f,0.016f,0.023f,1.0f);
    glClear(GL_COLOR_BUFFER_BIT|GL_DEPTH_BUFFER_BIT|GL_STENCIL_BUFFER_BIT);
    if(!eglSwapBuffers(g.display,g.surface)){
        g.error=egl_error("eglSwapBuffers");release_egl();return g.error;
    }
    ++g.frames;
    return "EGL frame aguardando DRAW/Xenos shaders";
}
} // namespace
namespace x360::gpu {
void set_video_options(int backend,int scale,bool vsync){
    std::lock_guard<std::mutex> lock(g.mutex);
    g.backend=backend;g.scale=scale;
    if(g.vsync!=vsync&&g.display!=EGL_NO_DISPLAY){
        g.vsync=vsync;
        eglSwapInterval(g.display,vsync?1:0);
    }else g.vsync=vsync;
}
std::string video_status(){
    std::lock_guard<std::mutex> lock(g.mutex);
    const auto seconds=std::max(0.001,std::chrono::duration<double>(Clock::now()-g.epoch).count());
    auto pm4=processor().stats();
    std::ostringstream out;
    out<<g.error<<"; EGL frames="<<g.frames<<"; FPS="<<int(g.frames/seconds)
       <<"; PM4="<<pm4.packets<<"; LOAD_ALU="<<pm4.alu_loads
       <<"; Xenos microcode uploads="<<pm4.shader_uploads
       <<"; DRAW pendentes="<<pm4.draws_seen
       <<"; unsupported="<<pm4.unsupported;
    if(!pm4.last_event.empty())out<<"; last="<<pm4.last_event;
    return out.str();
}
} // namespace x360::gpu
extern "C" JNIEXPORT jstring JNICALL
Java_com_x360nexus_MainActivity_nativeSetVideoSurface(JNIEnv* env,jobject,jobject surface){
    std::lock_guard<std::mutex> lock(g.mutex);
    release_egl();
    if(g.window){ANativeWindow_release(g.window);g.window=nullptr;}
    if(surface)g.window=ANativeWindow_fromSurface(env,surface);
    g.epoch=Clock::now();g.frames=0;
    g.error=g.window?"SurfaceView anexada":"SurfaceView liberada";
    return env->NewStringUTF(g.error.c_str());
}
extern "C" JNIEXPORT jstring JNICALL
Java_com_x360nexus_MainActivity_nativeRenderVideoFrame(JNIEnv* env,jobject){
    std::lock_guard<std::mutex> lock(g.mutex);
    const auto result=render();
    return env->NewStringUTF(result.c_str());
}
extern "C" JNIEXPORT jstring JNICALL
Java_com_x360nexus_MainActivity_nativeVideoStats(JNIEnv* env,jobject){
    const auto result=x360::gpu::video_status();
    return env->NewStringUTF(result.c_str());
}
