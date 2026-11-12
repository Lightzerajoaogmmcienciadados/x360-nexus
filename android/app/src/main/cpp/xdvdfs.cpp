#include "xdvdfs.hpp"

#include <algorithm>
#include <array>
#include <cerrno>
#include <cctype>
#include <cstring>
#include <limits>
#include <stdexcept>
#include <string>
#include <sys/stat.h>
#include <sys/types.h>
#include <unistd.h>
#include <unordered_set>

namespace x360::xdvdfs {
namespace {
constexpr uint64_t kSector = 2048;
constexpr uint64_t kDescriptorSector = 32;
constexpr size_t kMaxDirBytes = 16u * 1024 * 1024;
constexpr size_t kMaxEntries = 15000;
constexpr unsigned kMaxDepth = 24;
constexpr uint64_t kMaxMetadataBytes = 64ull * 1024 * 1024;
constexpr std::array<uint8_t,20> kMagic = {
    'M','I','C','R','O','S','O','F','T','*','X','B','O','X','*','M','E','D','I','A'};
uint16_t le16(const uint8_t* p) {
    return static_cast<uint16_t>(p[0] | static_cast<uint16_t>(p[1]) << 8);
}
uint32_t le32(const uint8_t* p) {
    return static_cast<uint32_t>(p[0]) | (static_cast<uint32_t>(p[1]) << 8) |
           (static_cast<uint32_t>(p[2]) << 16) | (static_cast<uint32_t>(p[3]) << 24);
}
std::string ascii_name(const uint8_t* data, size_t size) {
    std::string name;
    name.reserve(size);
    for (size_t i=0; i<size; ++i) {
        uint8_t c = data[i];
        if (c=='/' || c=='\\' || c==0) throw std::runtime_error("Nome XDVDFS com separador ou NUL invalido");
        name.push_back((c >= 32 && c < 127) ? static_cast<char>(c) : '_');
    }
    if (name=="." || name==".." || name.empty()) throw std::runtime_error("Nome XDVDFS invalido");
    return name;
}
bool lower_equal(const std::string& a, const char* b) {
    const std::string target(b);
    if (a.size() != target.size()) return false;
    for(size_t i=0; i<a.size(); ++i)
        if (std::tolower(static_cast<unsigned char>(a[i])) !=
            std::tolower(static_cast<unsigned char>(target[i]))) return false;
    return true;
}
void check_range(uint64_t offset, uint64_t size, uint64_t limit) {
    if(offset > limit || size > limit-offset)
        throw std::runtime_error("Acesso fora dos limites da ISO");
}
uint32_t crc32_update(uint32_t crc, const uint8_t* ptr, size_t size) {
    for(size_t i=0;i<size;i++) {
        crc ^= ptr[i];
        for(int j=0;j<8;j++) crc = (crc >> 1) ^ (0xEDB88320u & (0u - (crc & 1u)));
    }
    return crc;
}
} // namespace

FdReader::FdReader(int fd) : fd_(fd), size_(0) {
    if(fd < 0) throw std::runtime_error("Descritor de arquivo invalido");
    struct stat st{};
    if(fstat(fd, &st) != 0) throw std::runtime_error(std::string("fstat falhou: ") + std::strerror(errno));
    if(st.st_size > 0) size_ = static_cast<uint64_t>(st.st_size);
    else {
        off_t end = lseek(fd, 0, SEEK_END);
        if(end <= 0) throw std::runtime_error("Documento nao permite acesso aleatorio; selecione uma ISO local com suporte a seek");
        size_ = static_cast<uint64_t>(end);
    }
}
void FdReader::read(uint64_t offset, void* dst, size_t count) const {
    check_range(offset, count, size_);
    if(offset > static_cast<uint64_t>(std::numeric_limits<off_t>::max()))
        throw std::runtime_error("Offset ISO nao representavel neste sistema");
    auto* bytes = static_cast<uint8_t*>(dst);
    size_t done = 0;
    while(done < count) {
        ssize_t n = pread(fd_, bytes + done, count - done, static_cast<off_t>(offset + done));
        if(n < 0 && errno == EINTR) continue;
        if(n < 0) throw std::runtime_error(std::string("Erro lendo ISO (precisa seek): ") + std::strerror(errno));
        if(n == 0) throw std::runtime_error("Fim inesperado de ISO");
        done += static_cast<size_t>(n);
    }
}

uint64_t Image::sector_offset(uint32_t sector, uint64_t size) const {
    uint64_t offset = volume_.partition_base + static_cast<uint64_t>(sector) * kSector;
    check_range(offset, size, volume_.image_size);
    return offset;
}
Image::Image(const FdReader& reader) : reader_(reader) {
    // Known full-dump partition offsets and untrimmed XISO. No 8 GB scanning.
    const std::array<std::pair<const char*, uint64_t>,5> partitions = {{
        {"XISO",0}, {"XGD2",0x0FD90000ull}, {"XGD3",0x02080000ull},
        {"XGD1",0x18300000ull}, {"XGD2 hybrid",0x89D80000ull}}};
    for(const auto& [label,base] : partitions) {
        const uint64_t off = base + kDescriptorSector * kSector;
        if(off > reader.size() || kSector > reader.size()-off) continue;
        std::array<uint8_t, kSector> desc{};
        reader.read(off, desc.data(), desc.size());
        if(!std::equal(kMagic.begin(), kMagic.end(), desc.begin()) ||
           !std::equal(kMagic.begin(), kMagic.end(), desc.begin()+2028)) continue;
        const uint32_t root_sector = le32(desc.data()+0x14);
        const uint32_t root_size = le32(desc.data()+0x18);
        if(root_size > kMaxDirBytes || root_size == 0) continue;
        const uint64_t root_at = base + static_cast<uint64_t>(root_sector)*kSector;
        if(root_at > reader.size() || root_size > reader.size()-root_at) continue;
        volume_ = {label,base,root_sector,root_size,reader.size()};
        std::vector<uint64_t> ancestors;
        parse_directory(root_sector, root_size, "", 0, ancestors);
        return;
    }
    throw std::runtime_error("Descritor XDVDFS nao encontrado nos offsets XISO/XGD2/XGD3/XGD1");
}
void Image::parse_directory(uint32_t sector, uint32_t size, const std::string& prefix,
                            unsigned depth, std::vector<uint64_t>& ancestor_dirs) {
    if(size == 0) return;
    if(depth > kMaxDepth) throw std::runtime_error("Diretorios aninhados demais");
    if(size > kMaxDirBytes) throw std::runtime_error("Diretorio excede o tamanho permitido");
    const uint64_t at = sector_offset(sector,size);
    if(std::find(ancestor_dirs.begin(),ancestor_dirs.end(),at) != ancestor_dirs.end())
        throw std::runtime_error("Referencia circular de diretorio");
    // Bound total metadata read even if the ISO contains thousands of directories.
    if(metadata_bytes_ > kMaxMetadataBytes || size > kMaxMetadataBytes-metadata_bytes_)
        throw std::runtime_error("Limite total de metadados XDVDFS excedido");
    metadata_bytes_ += size;
    if(entries_.size() >= kMaxEntries) throw std::runtime_error("Muitas entradas no XDVDFS");
    ancestor_dirs.push_back(at);
    struct Guard { std::vector<uint64_t>& v; ~Guard(){v.pop_back();} } guard{ancestor_dirs};
    std::vector<uint8_t> data(size);
    reader_.read(at, data.data(),size);
    std::vector<uint32_t> tree_nodes{0};
    std::unordered_set<uint32_t> visited;
    struct Child {uint32_t sector;uint32_t size;std::string path;};
    std::vector<Child> child_dirs;
    while(!tree_nodes.empty()) {
        uint32_t pos = tree_nodes.back(); tree_nodes.pop_back();
        if(!visited.insert(pos).second) throw std::runtime_error("Ciclo na arvore do diretorio XDVDFS");
        if(visited.size()>kMaxEntries || entries_.size()>=kMaxEntries)
            throw std::runtime_error("Muitas entradas XDVDFS");
        if(pos >= data.size() || data.size()-pos < 14)
            throw std::runtime_error("Registro de diretorio fora dos limites");
        const uint8_t* p = data.data()+pos;
        const uint32_t left = static_cast<uint32_t>(le16(p))*4;
        const uint32_t right= static_cast<uint32_t>(le16(p+2))*4;
        const uint32_t entry_sector = le32(p+4);
        const uint32_t entry_size = le32(p+8);
        const bool directory=(p[12]&0x10u)!=0;
        const uint8_t len=p[13];
        if(len == 0 || static_cast<size_t>(len) > data.size()-pos-14)
            throw std::runtime_error("Nome de registro XDVDFS invalido");
        std::string name=ascii_name(p+14,len);
        std::string path=prefix + name;
        if(path.size()>2048) throw std::runtime_error("Caminho XDVDFS longo demais");
        if(entry_size) sector_offset(entry_sector,entry_size);
        entries_.push_back({path,entry_sector,entry_size,directory});
        if(directory && entry_size) child_dirs.push_back({entry_sector,entry_size,path+"/"});
        if(right) tree_nodes.push_back(right);
        if(left) tree_nodes.push_back(left);
    }
    for(const auto& d:child_dirs) parse_directory(d.sector,d.size,d.path,depth+1,ancestor_dirs);
}
const Entry* Image::find_default_xex() const {
    for(const auto& e:entries_) if(!e.directory && lower_equal(e.path,"default.xex")) return &e;
    for(const auto& e:entries_) {
        if(e.directory) continue;
        const auto slash=e.path.find_last_of('/');
        const std::string basename=(slash == std::string::npos) ? e.path : e.path.substr(slash+1);
        if(lower_equal(basename,"default.xex")) return &e;
    }
    return nullptr;
}
void Image::read_file(const Entry& file, uint64_t offset, void* dst, size_t size) const {
    if(file.directory) throw std::runtime_error("Nao e possivel ler diretorio como arquivo");
    check_range(offset,size,file.size);
    const uint64_t at=sector_offset(file.sector,file.size);
    reader_.read(at+offset,dst,size);
}
uint64_t GuestRam::checked_offset(uint32_t address, size_t size) {
    if(address < kGuestRamBase) throw std::runtime_error("Endereco antes da RAM emulada");
    const uint64_t offset=static_cast<uint64_t>(address)-kGuestRamBase;
    if(offset > kRamSize || size > kRamSize-offset) throw std::runtime_error("Endereco fora de 512 MiB");
    return offset;
}
void GuestRam::write(uint32_t guest_address, const void* src, size_t size) {
    const uint64_t at=checked_offset(guest_address,size);
    const auto* b=static_cast<const uint8_t*>(src);
    size_t copied=0;
    while(copied<size) {
        const uint64_t cur=at+copied;
        const uint32_t page=static_cast<uint32_t>(cur/kPageSize);
        const size_t local=static_cast<size_t>(cur%kPageSize);
        const size_t take=std::min(size-copied,kPageSize-local);
        auto& owner=pages_[page];
        if(!owner) owner=std::make_unique<Page>();
        std::memcpy(owner->data+local,b+copied,take);
        copied+=take;
    }
}
void GuestRam::read(uint32_t guest_address, void* dst, size_t size) const {
    const uint64_t at=checked_offset(guest_address,size);
    auto* b=static_cast<uint8_t*>(dst);
    size_t copied=0;
    while(copied<size) {
        const uint64_t cur=at+copied;
        const uint32_t page=static_cast<uint32_t>(cur/kPageSize);
        const size_t local=static_cast<size_t>(cur%kPageSize);
        const size_t take=std::min(size-copied,kPageSize-local);
        const auto it=pages_.find(page);
        if(it==pages_.end()) std::memset(b+copied,0,take);
        else std::memcpy(b+copied,it->second->data+local,take);
        copied+=take;
    }
}
LoadResult load_xex(const Image& image, const Entry& file, GuestRam& ram) {
    if(file.directory || file.size<4) throw std::runtime_error("default.xex vazio ou invalido");
    if(file.size>kMaxLoadedXex) throw std::runtime_error("XEX maior que limite seguro de 128 MiB para carregamento em RAM");
    constexpr size_t kChunk=64*1024;
    std::array<uint8_t,kChunk> chunk{};
    uint32_t crc=0xFFFFFFFFu;
    bool xex=false;
    for(uint64_t i=0;i<file.size;) {
        const auto n=static_cast<size_t>(std::min<uint64_t>(chunk.size(),file.size-i));
        image.read_file(file,i,chunk.data(),n);
        if(i==0) xex=std::memcmp(chunk.data(),"XEX2",4)==0;
        crc=crc32_update(crc,chunk.data(),n);
        ram.write(static_cast<uint32_t>(kGuestRamBase+i),chunk.data(),n);
        i+=n;
    }
    if(!xex) throw std::runtime_error("default.xex nao comeca com assinatura XEX2");
    std::array<uint8_t,4> verify{};
    ram.read(kGuestRamBase,verify.data(),verify.size());
    if(std::memcmp(verify.data(),"XEX2",4)!=0) throw std::runtime_error("Falha ao verificar bytes na memoria emulada");
    return {kGuestRamBase,file.size,~crc,xex};
}
uint64_t copy_file_to_fd(const Image& image, const Entry& file, int output_fd) {
    if(file.directory || output_fd < 0) throw std::runtime_error("Destino de extracao invalido");
    // Explicit size limit prevents writing unexpectedly enormous files from malformed ISOs.
    if(file.size > 512u*1024*1024) throw std::runtime_error("Arquivo grande demais para exportar (limite 512 MiB)");
    std::array<uint8_t,64*1024> block{};
    uint64_t done=0;
    while(done<file.size) {
        const size_t n=static_cast<size_t>(std::min<uint64_t>(block.size(),file.size-done));
        image.read_file(file,done,block.data(),n);
        size_t written=0;
        while(written<n) {
            ssize_t r=::write(output_fd,block.data()+written,n-written);
            if(r<0 && errno==EINTR) continue;
            if(r<=0) throw std::runtime_error("Nao foi possivel gravar arquivo extraido");
            written+=static_cast<size_t>(r);
        }
        done+=n;
    }
    return done;
}
} // namespace x360::xdvdfs
