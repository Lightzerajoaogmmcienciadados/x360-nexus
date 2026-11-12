#include "xdvdfs.hpp"
#include "xex_loader.hpp"
#include <jni.h>

#include <cstdio>
#include <array>
#include <algorithm>
#include <vector>
#include <optional>
#include <exception>
#include <memory>
#include <mutex>
#include <sstream>
#include <string>

using x360::xdvdfs::Entry;
using x360::xdvdfs::FdReader;
using x360::xdvdfs::GuestRam;
using x360::xdvdfs::Image;

namespace {
std::mutex g_memory_mutex;
std::unique_ptr<GuestRam> g_ram;
uint32_t g_loaded_bytes=0;
uint32_t g_loaded_crc=0;
std::string g_loaded_path;
std::vector<uint8_t> g_raw_xex; // Original staged bytes, retained after PE mapping.
std::optional<x360::xex::PreparedImage> g_prepared;
// In-memory only. Never saved, logged or embedded in the distributed APK.
std::optional<std::array<uint8_t,16>> g_external_master_key;

jstring java_string(JNIEnv* env, const std::string& s) {
    // Paths are ASCII-sanitized by the XDVDFS reader; avoid embedded NUL.
    return env->NewStringUTF(s.c_str());
}
std::string format_hex(uint64_t value) {
    char result[32];
    std::snprintf(result,sizeof(result),"0x%llX",static_cast<unsigned long long>(value));
    return result;
}
const Entry& default_xex(const Image& image) {
    const Entry* e=image.find_default_xex();
    if(!e) throw std::runtime_error("default.xex nao encontrado nesta ISO");
    return *e;
}
} // namespace

extern "C" JNIEXPORT jstring JNICALL
Java_com_x360nexus_MainActivity_nativeSetXexKey(JNIEnv* env, jobject, jbyteArray key_bytes) {
    if(!key_bytes || env->GetArrayLength(key_bytes)!=16)
        return java_string(env,"keys.bin deve conter exatamente 16 bytes binarios; nada foi alterado.");
    std::array<uint8_t,16> new_key{};
    env->GetByteArrayRegion(key_bytes,0,16,reinterpret_cast<jbyte*>(new_key.data()));
    if(env->ExceptionCheck()){
        std::fill(new_key.begin(),new_key.end(),0);
        return nullptr;
    }
    {
        std::lock_guard<std::mutex> lock(g_memory_mutex);
        if(g_external_master_key)std::fill(g_external_master_key->begin(),g_external_master_key->end(),0);
        g_external_master_key=new_key;
        g_prepared.reset();
    }
    std::fill(new_key.begin(),new_key.end(),0);
    return java_string(env,"Chave externa carregada em memoria (16 bytes). Valor nao exibido nem salvo.");
}

extern "C" JNIEXPORT jstring JNICALL
Java_com_x360nexus_MainActivity_nativeClearXexKey(JNIEnv* env, jobject) {
    std::lock_guard<std::mutex> lock(g_memory_mutex);
    if(g_external_master_key){
        std::fill(g_external_master_key->begin(),g_external_master_key->end(),0);
        g_external_master_key.reset();
    }
    g_prepared.reset();
    return java_string(env,"Chave removida da memoria do aplicativo.");
}

extern "C" JNIEXPORT jstring JNICALL
Java_com_x360nexus_MainActivity_nativeInspectXdvdfs(JNIEnv* env, jobject, jint fd) {
    try {
        const FdReader reader(fd);
        const Image image(reader);
        const auto& v=image.volume();
        std::ostringstream out;
        out << "XDVDFS reconhecido: " << v.layout << "\n"
            << "Particao: " << format_hex(v.partition_base) << "\n"
            << "ISO: " << v.image_size << " bytes\n"
            << "Diretorio raiz: setor " << v.root_sector << "\n"
            << "Entradas encontradas: " << image.entries().size() << "\n";
        if(const Entry* found=image.find_default_xex())
            out << "default.xex localizado: " << found->path << " (" << found->size << " bytes)\n";
        else out << "default.xex nao encontrado\n";
        out << "\nARQUIVOS:\n";
        constexpr size_t kShown=350;
        size_t i=0;
        for(const Entry& e:image.entries()) {
            if(i++>=kShown) {out << "... listagem limitada aos primeiros " << kShown << " itens\n"; break;}
            out << (e.directory ? "[DIR] " : "[FILE] ") << e.path
                << (e.directory ? "" : " ("+std::to_string(e.size)+" bytes)") << "\n";
        }
        return java_string(env,out.str());
    } catch(const std::exception& e) {
        return java_string(env,std::string("Erro lendo XDVDFS: ")+e.what());
    }
}

extern "C" JNIEXPORT jstring JNICALL
Java_com_x360nexus_MainActivity_nativeLoadDefaultXex(JNIEnv* env, jobject, jint fd) {
    try {
        const FdReader reader(fd);
        const Image image(reader);
        const Entry& file=default_xex(image);
        auto next_ram=std::make_unique<GuestRam>();
        const auto loaded=x360::xdvdfs::load_xex(image,file,*next_ram);
        const size_t committed=next_ram->allocated_bytes();
        {
            std::lock_guard<std::mutex> lock(g_memory_mutex);
            g_ram=std::move(next_ram);
            g_loaded_path=file.path;
            g_loaded_bytes=loaded.loaded_bytes;
            g_loaded_crc=loaded.crc32;
            g_raw_xex.clear();
            g_prepared.reset();
        }
        return java_string(env,"default.xex armazenado na RAM emulada!\n"
            "Arquivo: "+file.path+"\n"
            "Endereco de staging: "+format_hex(loaded.guest_address)+"\n"
            "Bytes carregados: "+std::to_string(loaded.loaded_bytes)+"\n"
            "RAM fisica alocada: "+std::to_string(committed)+" bytes\n"
            "CRC32: "+format_hex(loaded.crc32)+"\n"
            "Assinatura: XEX2\n\n"
            "AVISO: imagem XEX copiada para area de staging; ainda nao foi "
            "descriptografada, relocada ou executada pela CPU PowerPC.");
    } catch(const std::exception& e) {
        return java_string(env,std::string("Erro carregando default.xex: ")+e.what());
    }
}

extern "C" JNIEXPORT jstring JNICALL
Java_com_x360nexus_MainActivity_nativeSaveDefaultXex(JNIEnv* env, jobject, jint input_fd, jint output_fd) {
    try {
        const FdReader reader(input_fd);
        const Image image(reader);
        const Entry& file=default_xex(image);
        const auto total=x360::xdvdfs::copy_file_to_fd(image,file,output_fd);
        return java_string(env,"default.xex extraido para o arquivo escolhido.\nBytes gravados: "+std::to_string(total));
    } catch(const std::exception& e) {
        return java_string(env,std::string("Erro extraindo default.xex: ")+e.what());
    }
}

extern "C" JNIEXPORT jstring JNICALL
Java_com_x360nexus_MainActivity_nativeGuestRamStatus(JNIEnv* env, jobject) {
    std::lock_guard<std::mutex> lock(g_memory_mutex);
    if(!g_ram) return java_string(env,"Nenhum default.xex carregado na RAM emulada.");
    uint8_t header[4]{};
    g_ram->read(x360::xdvdfs::kGuestRamBase,header,sizeof(header));
    return java_string(env,"RAM virtual: 512 MiB\n"
        "Base: "+format_hex(x360::xdvdfs::kGuestRamBase)+"\n"
        "Arquivo: "+g_loaded_path+"\n"
        "Bytes em staging: "+std::to_string(g_loaded_bytes)+"\n"
        "Host RAM utilizada: "+std::to_string(g_ram->allocated_bytes())+" bytes\n"
        "CRC32: "+format_hex(g_loaded_crc)+"\n"
        "Primeira palavra big-endian: "+format_hex((uint32_t(header[0])<<24) | (uint32_t(header[1])<<16) | (uint32_t(header[2])<<8) | header[3]));
}

// v4.3 — metadata of the XEX2 already stored in the shared C++ guest RAM.
extern "C" JNIEXPORT jstring JNICALL
Java_com_x360nexus_MainActivity_nativeInspectStagedXex(JNIEnv* env, jobject) {
    try {
        std::lock_guard<std::mutex> lock(g_memory_mutex);
        if(!g_ram || !g_loaded_bytes) return java_string(env,"Primeiro use 3. CARREGAR default.xex NA RAM EMULADA");
        if(g_raw_xex.empty()) {
            g_raw_xex.resize(g_loaded_bytes);
            g_ram->read(x360::xdvdfs::kGuestRamBase,g_raw_xex.data(),g_raw_xex.size());
        }
        const auto h=x360::xex::parse_headers(g_raw_xex);
        return java_string(env,x360::xex::describe(h));
    } catch(const std::exception& e) {
        return java_string(env,std::string("Falha analisando XEX2: ")+e.what());
    }
}

extern "C" JNIEXPORT jstring JNICALL
Java_com_x360nexus_MainActivity_nativePrepareStagedXex(JNIEnv* env, jobject) {
    try {
        std::lock_guard<std::mutex> lock(g_memory_mutex);
        if(!g_ram || !g_loaded_bytes) return java_string(env,"Primeiro carregue default.xex pelo botao 3.");
        if(g_raw_xex.empty()) {
            g_raw_xex.resize(g_loaded_bytes);
            g_ram->read(x360::xdvdfs::kGuestRamBase,g_raw_xex.data(),g_raw_xex.size());
        }
        const auto headers=x360::xex::parse_headers(g_raw_xex);
        auto pe=x360::xex::extract_pe(g_raw_xex,headers,g_external_master_key);
        if(pe.size()>x360::xex::kMaxPeBytes) throw std::runtime_error("PE fora do limite");
        auto next=std::make_unique<GuestRam>();
        x360::xex::PreparedImage prepared;
        try {prepared=x360::xex::map_pe(pe,headers,*next);}
        catch (...) {std::fill(pe.begin(),pe.end(),0);throw;}
        std::fill(pe.begin(),pe.end(),0); // discard host-side extracted image after staging
        g_prepared=std::move(prepared);
        g_ram=std::move(next);
        return java_string(env,"PE32 POWERPCBE preparado na RAM!\nBase: "+format_hex(g_prepared->base)+
            "\nEntry Point: "+format_hex(g_prepared->pc)+
            "\nSecoes: "+std::to_string(g_prepared->sections.size())+
            "\nBibliotecas importadas: "+std::to_string(g_prepared->imports.size())+
            "\nImports xboxkrnl/xam: pendentes de resolucao real.\n"
            "CPU Rust limitada a instrucoes implementadas; sem GPU/kernel Xbox 360 completos.");
    } catch(const std::exception& e) {
        {
            std::lock_guard<std::mutex> lock(g_memory_mutex);
            g_prepared.reset();
        }
        return java_string(env,std::string("XEX2 identificado, mas PE nao preparado: ")+e.what());
    }
}

extern "C" JNIEXPORT jlong JNICALL
Java_com_x360nexus_MainActivity_nativePreparedEntryPoint(JNIEnv*, jobject) {
    std::lock_guard<std::mutex> lock(g_memory_mutex);
    return g_prepared?static_cast<jlong>(g_prepared->pc):0;
}

extern "C" JNIEXPORT jbyteArray JNICALL
Java_com_x360nexus_MainActivity_nativeFetchCodeWindow(JNIEnv* env, jobject, jlong address, jint length) {
    std::lock_guard<std::mutex> lock(g_memory_mutex);
    if(!g_prepared || !g_ram || address<0 || address>UINT32_MAX || length<=0 || length>4096 || (length%4))
        return nullptr;
    const auto at=static_cast<uint32_t>(address);
    if(!g_prepared->can_fetch(at,static_cast<size_t>(length)))return nullptr;
    std::vector<uint8_t> bytes(static_cast<size_t>(length));
    try {g_ram->read(at,bytes.data(),bytes.size());}
    catch (...) {return nullptr;}
    auto array=env->NewByteArray(length);
    if(!array)return nullptr;
    env->SetByteArrayRegion(array,0,length,reinterpret_cast<const jbyte*>(bytes.data()));
    return array;
}