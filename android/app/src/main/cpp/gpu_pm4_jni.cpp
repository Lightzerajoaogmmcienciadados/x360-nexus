#include "xenos_pm4.hpp"
#include "xdvdfs.hpp"
#include <jni.h>
#include <string>
#include <sstream>
#include <mutex>
#include <array>
#include <vector>
#include <cstdint>

// PM4 source currently comes from an explicitly selected guest RAM address.
// No Xbox 360 GPU register/MMIO DMA interception is claimed.
extern "C" bool x360_try_read_guest_ram(uint32_t address, void* dst, size_t len);
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
    const bool ok=x360::gpu::processor().submit(words,error);
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