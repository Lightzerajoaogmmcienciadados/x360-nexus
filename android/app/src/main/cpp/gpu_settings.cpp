// v4.5 GPU configuration bridge. Stores capability observations and user
// preferences; it does NOT implement Xenos shader translation or custom drivers.
#include <jni.h>
#include <mutex>
#include <string>
#include <algorithm>
namespace {
std::mutex config_lock;
struct Settings {
    int backend=0; // 0=Auto,1=Vulkan,2=GLES
    int resolution=1; // 0=0.5x,1=1x,2=1.5x,3=2x
    bool turnip=false,exact_shader=false,shader_cache=true,vsync=true,show_fps=true;
    std::string renderer="GPU ainda nao detectada";
    std::string gles="nao detectado",vulkan="nao detectado";
} cfg;
std::string read(JNIEnv* e,jstring s){if(!s)return "";const char* p=e->GetStringUTFChars(s,nullptr);if(!p)return "";std::string out(p);e->ReleaseStringUTFChars(s,p);return out.substr(0,256);}
jstring make(JNIEnv* e,const std::string& s){return e->NewStringUTF(s.c_str());}
}
extern "C" JNIEXPORT jstring JNICALL
Java_com_x360nexus_MainActivity_nativeSetGpuCapabilities(JNIEnv* e,jobject,jstring renderer,jstring gles,jstring vk){
    std::lock_guard<std::mutex> lk(config_lock);
    cfg.renderer=read(e,renderer);cfg.gles=read(e,gles);cfg.vulkan=read(e,vk);
    return make(e,"GPU: "+cfg.renderer+" | GLES: "+cfg.gles+" | Vulkan: "+cfg.vulkan);
}
extern "C" JNIEXPORT jstring JNICALL
Java_com_x360nexus_MainActivity_nativeSetGraphicsSettings(JNIEnv* e,jobject,jint backend,jint resolution,jboolean turnip,jboolean exact,jboolean cache,jboolean vsync,jboolean fps){
    if(backend<0||backend>2||resolution<0||resolution>3)return make(e,"Configuracoes invalidas");
    std::lock_guard<std::mutex> lk(config_lock);
    cfg.backend=backend;cfg.resolution=resolution;
    // Do not pretend to install or enable Turnip: no alternative driver bundled.
    cfg.turnip=false;cfg.exact_shader=exact;cfg.shader_cache=cache;cfg.vsync=vsync;cfg.show_fps=fps;
    const char* names[]={"Auto","Vulkan solicitado","OpenGL ES solicitado"};
    std::string response=std::string("Preferencias registradas: ")+names[backend]+", escala "+
        (resolution==0?"0.5x":resolution==1?"1x":resolution==2?"1.5x":"2x");
    if(turnip)response+=" | Turnip NAO disponivel (sem driver embutido)";
    response+=" | renderizador Xenos NAO implementado";
    return make(e,response);
}