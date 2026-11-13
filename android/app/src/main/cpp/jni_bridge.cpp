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
std::optional<x360::xex::ImportInventory> g_import_inventory;
size_t g_patched_thunks=0;
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
        g_import_inventory.reset();
        g_patched_thunks=0;
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
            g_import_inventory.reset();
            g_patched_thunks=0;
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
        g_import_inventory=x360::xex::inspect_import_thunks(g_prepared->imports,
            g_prepared->base,g_prepared->image_size,*g_ram);
        g_patched_thunks=x360::xex::patch_import_thunks(*g_import_inventory,
            g_prepared->executable_ranges,*g_ram);
        return java_string(env,"PE32 POWERPCBE preparado na RAM!\nBase: "+format_hex(g_prepared->base)+
            "\nEntry Point: "+format_hex(g_prepared->pc)+
            "\nSecoes: "+std::to_string(g_prepared->sections.size())+
            "\nBibliotecas importadas: "+std::to_string(g_prepared->imports.size())+
            "\n"+g_import_inventory->describe()+
            "\nThunks HLE de tipo 1 aplicados em seções executáveis: "+std::to_string(g_patched_thunks)+"\n"
            "CPU Rust interpreta somente instrucoes conhecidas; a GPU Xenos ainda nao traduz shaders comerciais.");
    } catch(const std::exception& e) {
        {
            std::lock_guard<std::mutex> lock(g_memory_mutex);
            g_prepared.reset();
            g_import_inventory.reset();
            g_patched_thunks=0;
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

// v4.5 diagnostic snapshot. Rust executes this private COPY of the PE32 image;
// no 512 MiB allocation and no mutable cross-language aliasing occurs.
extern "C" JNIEXPORT jlongArray JNICALL
Java_com_x360nexus_MainActivity_nativePreparedInfo(JNIEnv* env,jobject){
    std::lock_guard<std::mutex> lock(g_memory_mutex);
    if(!g_prepared || !g_ram)return nullptr;
    const jlong data[]{static_cast<jlong>(g_prepared->base),static_cast<jlong>(g_prepared->pc),
        static_cast<jlong>(g_prepared->image_size),static_cast<jlong>(g_prepared->imports.size()),
        static_cast<jlong>(g_ram->allocated_bytes())};
    jlongArray out=env->NewLongArray(5);
    if(out)env->SetLongArrayRegion(out,0,5,data);
    return out;
}
extern "C" JNIEXPORT jlongArray JNICALL
Java_com_x360nexus_MainActivity_nativeExecutableRanges(JNIEnv* env,jobject){
    std::lock_guard<std::mutex> lock(g_memory_mutex);
    if(!g_prepared || g_prepared->executable_ranges.size()>128)return nullptr;
    std::vector<jlong> values;
    for(const auto& section:g_prepared->executable_ranges){
        values.push_back(section.first);values.push_back(section.second);
    }
    jlongArray out=env->NewLongArray(static_cast<jsize>(values.size()));
    if(out&&!values.empty())env->SetLongArrayRegion(out,0,static_cast<jsize>(values.size()),values.data());
    return out;
}
extern "C" JNIEXPORT jbyteArray JNICALL
Java_com_x360nexus_MainActivity_nativePreparedSnapshot(JNIEnv* env,jobject){
    std::lock_guard<std::mutex> lock(g_memory_mutex);
    constexpr uint32_t max_snapshot=16*1024*1024;
    if(!g_ram || !g_prepared || g_prepared->image_size==0 || g_prepared->image_size>max_snapshot)return nullptr;
    const auto size=g_prepared->image_size;
    try{
        std::vector<uint8_t> copy(size);
        g_ram->read(g_prepared->base,copy.data(),copy.size());
        jbyteArray out=env->NewByteArray(static_cast<jsize>(size));
        if(out)env->SetByteArrayRegion(out,0,static_cast<jsize>(size),reinterpret_cast<const jbyte*>(copy.data()));
        return out;
    }catch(...){return nullptr;}
}

// Read-only bridge for the PM4 diagnostic decoder. Do not hold this mutex
// while calling GPU or EGL methods, avoiding the video-lock hierarchy.
extern "C" bool x360_try_read_guest_ram(uint32_t address, void* dst, size_t len) {
    try {
        std::lock_guard<std::mutex> lock(g_memory_mutex);
        if (!g_ram || !g_prepared || len > 16384) return false;
        // Limit reads to a mapped image, never unvalidated arbitrary offsets.
        const uint64_t at=address, lo=g_prepared->base;
        if (at<lo || at+len>lo+g_prepared->image_size) return false;
        g_ram->read(address,dst,len);
        return true;
    } catch (...) {return false;}
}

// v5.1 streaming bridge. Each JNI transfer is at most 64 KiB: no 16-MiB
// Java snapshot and no 512-MiB dense guest RAM allocation are required.
// Reads are permitted only inside a successfully mapped PE32 image.
extern "C" JNIEXPORT jbyteArray JNICALL
Java_com_x360nexus_MainActivity_nativePreparedChunk(JNIEnv* env,jobject,jlong address,jint count){
    if(address<0 || address>UINT32_MAX || count<=0 || count>65536) return nullptr;
    std::vector<uint8_t> buffer(static_cast<size_t>(count));
    {
        std::lock_guard<std::mutex> lock(g_memory_mutex);
        if(!g_prepared || !g_ram)return nullptr;
        const uint64_t start=static_cast<uint64_t>(address);
        const uint64_t lo=g_prepared->base;
        const uint64_t hi=lo+g_prepared->image_size;
        if(start<lo || start+static_cast<uint64_t>(count)>hi)return nullptr;
        try {g_ram->read(static_cast<uint32_t>(address),buffer.data(),buffer.size());}
        catch (...){return nullptr;}
    }
    jbyteArray result=env->NewByteArray(count);
    if(result)env->SetByteArrayRegion(result,0,count,reinterpret_cast<const jbyte*>(buffer.data()));
    std::fill(buffer.begin(),buffer.end(),0);
    return result;
}

// Import catalog: triples [thunk guest address, module id, ordinal]. Only real
// executable thunk records are returned; no fake successful HLE mappings.
extern "C" JNIEXPORT jlongArray JNICALL
Java_com_x360nexus_MainActivity_nativeImportThunks(JNIEnv* env,jobject){
    std::lock_guard<std::mutex> lock(g_memory_mutex);
    if(!g_prepared || !g_ram)return nullptr;
    if(!g_import_inventory)return nullptr;
    const auto& inv=*g_import_inventory;
    if(inv.thunks.size()>8192)return nullptr;
    std::vector<jlong> triples;triples.reserve(inv.thunks.size()*3);
    for(const auto& thunk:inv.thunks){
        triples.push_back(thunk.address);
        triples.push_back(thunk.module);
        triples.push_back(thunk.ordinal);
    }
    auto out=env->NewLongArray(static_cast<jsize>(triples.size()));
    if(out && !triples.empty())env->SetLongArrayRegion(out,0,
        static_cast<jsize>(triples.size()),triples.data());
    return out;
}