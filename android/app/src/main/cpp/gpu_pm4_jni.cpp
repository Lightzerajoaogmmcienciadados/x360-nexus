#include "xenos_pm4.hpp"
#include "xdvdfs.hpp"
#include <jni.h>
#include <string>
#include <sstream>
#include <mutex>
#include <array>
#include <vector>
#include <limits>
#include <cstdint>

// PM4 source currently comes from an explicitly selected guest RAM address.
// No Xbox 360 GPU register/MMIO DMA interception is claimed.
extern "C" bool x360_try_read_guest_ram(uint32_t address, void* dst, size_t len);
// Treat the Xenos physical 30-bit field as a region of the 0x80000000 guest
// segment. Only mapped, read-only C++ guest memory is exposed to this callback.
static bool read_guest_pm4_words(uint32_t physical,size_t count,std::vector<uint32_t>& out){
    if(count==0||count>256||physical>0x3FFFFFFFu || uint64_t(physical)+count*4>0x40000000ull) return false;
    const uint32_t address=physical|0x80000000u;
    std::vector<uint8_t> bytes(count*4);
    if(!x360_try_read_guest_ram(address,bytes.data(),bytes.size()))return false;
    out.clear();out.reserve(count);
    for(size_t i=0;i<bytes.size();i+=4)
        out.push_back(uint32_t(bytes[i])<<24|uint32_t(bytes[i+1])<<16|uint32_t(bytes[i+2])<<8|bytes[i+3]);
    return true;
}

// Read additional guest PM4 data/shader constants from the active Rust CPU
// session, not from the earlier staging PE. JNI access stays synchronous and
// bounded to 16 KiB, never crossing threads with borrowed VM references.
static bool read_active_pm4_words(JNIEnv* env,jobject activity,
                                   uint32_t physical,size_t count,
                                   std::vector<uint32_t>& out){
    if(count==0 || count>4096 || physical>0x3FFFFFFFu ||
       uint64_t(physical)+count*4>0x40000000ull)return false;
    const uint32_t virtual_addr=physical|0x80000000u;
    auto cls=env->GetObjectClass(activity);
    if(!cls)return false;
    const auto method=env->GetMethodID(cls,"nativeReadSessionMemory","(JI)[B");
    env->DeleteLocalRef(cls);
    if(!method){
        if(env->ExceptionCheck())env->ExceptionClear();
        return false;
    }
    const auto n=static_cast<jint>(count*4);
    auto data=static_cast<jbyteArray>(env->CallObjectMethod(
        activity,method,static_cast<jlong>(virtual_addr),n));
    if(env->ExceptionCheck()){
        env->ExceptionClear();
        return false;
    }
    if(!data)return false;
    const bool valid=env->GetArrayLength(data)==n;
    std::vector<uint8_t> bytes(valid?size_t(n):0);
    if(valid)env->GetByteArrayRegion(data,0,n,
        reinterpret_cast<jbyte*>(bytes.data()));
    env->DeleteLocalRef(data);
    if(!valid||env->ExceptionCheck()){if(env->ExceptionCheck())env->ExceptionClear();return false;}
    out.clear();out.reserve(count);
    for(size_t i=0;i<bytes.size();i+=4)
        out.push_back(uint32_t(bytes[i])<<24|uint32_t(bytes[i+1])<<16|
                      uint32_t(bytes[i+2])<<8|bytes[i+3]);
    return true;
}
// Raw BE DWORD stream supplied from the *active Rust CPU RAM*, not the stale
// C++ XEX stage. Does not imply that hardware MMIO interception exists.
extern "C" JNIEXPORT jstring JNICALL
Java_com_x360nexus_MainActivity_nativeSubmitPm4Bytes(JNIEnv* env,jobject activity,jbyteArray raw){
    if(!raw)return env->NewStringUTF("PM4: stream nulo");
    const jsize n=env->GetArrayLength(raw);
    if(n<=0||n%4||n>16384)return env->NewStringUTF("PM4: stream precisa ter de 1 a 4096 DWORDs BE");
    std::vector<uint8_t> data(size_t(n),0);
    env->GetByteArrayRegion(raw,0,n,reinterpret_cast<jbyte*>(data.data()));
    if(env->ExceptionCheck())return nullptr;
    std::vector<uint32_t> words;words.reserve(data.size()/4);
    for(size_t i=0;i<data.size();i+=4)
        words.push_back(uint32_t(data[i])<<24|uint32_t(data[i+1])<<16|uint32_t(data[i+2])<<8|data[i+3]);
    std::string error;
    const x360::gpu::CommandProcessor::GuestWords reader=
        [env,activity](uint32_t addr,size_t count,std::vector<uint32_t>& dest) {
            return read_active_pm4_words(env,activity,addr,count,dest);
        };
    const bool ok=x360::gpu::processor().submit(words,error,reader);
    const auto stats=x360::gpu::processor().stats();
    std::ostringstream result;
    result<<(ok?"PM4 guest RAM: ":"PM4 rejeitado: ")<<(ok?stats.last_event:error)
          <<" | pacotes="<<stats.packets<<" LOAD_ALU="<<stats.alu_loads
          <<" uploads de microcode="<<stats.shader_uploads
          <<" DRAW pendentes="<<stats.draws_seen<<" unsupported="<<stats.unsupported;
    return env->NewStringUTF(result.str().c_str());
}

extern "C" JNIEXPORT jstring JNICALL
Java_com_x360nexus_MainActivity_nativeProcessPm4(JNIEnv* env,jobject,jlong address,jint dwords){
    if(address<0||address>UINT32_MAX||dwords<1||dwords>4096) return env->NewStringUTF("PM4: endereco ou quantidade invalida");
    std::vector<uint8_t> bytes(size_t(dwords)*4);
    if(!x360_try_read_guest_ram(static_cast<uint32_t>(address),bytes.data(),bytes.size()))
        return env->NewStringUTF("PM4: endereco de RAM nao disponivel; nenhum pacote lido");
    std::vector<uint32_t> words;
    words.reserve(size_t(dwords));
    for(size_t i=0;i<bytes.size();i+=4)words.push_back(uint32_t(bytes[i])<<24|uint32_t(bytes[i+1])<<16|uint32_t(bytes[i+2])<<8|bytes[i+3]);
    std::string error;
    const bool ok=x360::gpu::processor().submit(words,error,read_guest_pm4_words);
    auto s=x360::gpu::processor().stats();
    std::ostringstream out;
    out<<(ok?"PM4 analisado: ":"PM4 rejeitado: ")<<(ok?s.last_event:error)
       <<"\nPacotes="<<s.packets<<" writes="<<s.register_writes<<" draws="<<s.draws_seen<<" unsupported="<<s.unsupported;
    return env->NewStringUTF(out.str().c_str());
}

extern "C" JNIEXPORT jstring JNICALL
Java_com_x360nexus_MainActivity_nativeSubmitDemoPm4(JNIEnv* env,jobject){
    // Real Xenos opcode SET_CONSTANT (0x2D), but synthetic constant data.
    const std::array<uint32_t,3> packet{(3u<<30)|(2u-1u)<<16|(0x2du<<8),(4u<<16)|5u,0x3F800000u};
    std::string error;
    const bool ok=x360::gpu::processor().submit(packet,error);
    const auto stats=x360::gpu::processor().stats();
    std::ostringstream out;
    out<<"PM4 sintético: "<<(ok?"SET_CONSTANT registrado":"erro: "+error)
       <<"; pacotes="<<stats.packets<<", writes="<<stats.register_writes
       <<", desenhos nao traduzidos="<<stats.draws_seen;
    return env->NewStringUTF(out.str().c_str());
}