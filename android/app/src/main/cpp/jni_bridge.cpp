#include "xdvdfs.hpp"
#include <jni.h>

#include <cstdio>
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
        "Leitura de verificacao: "+std::string(header,header+4));
}
