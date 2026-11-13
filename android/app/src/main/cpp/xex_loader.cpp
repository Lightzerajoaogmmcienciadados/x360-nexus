#include "xex_loader.hpp"
#include <algorithm>
#include <array>
#include <cstring>
#include <iomanip>
#include <limits>
#include <sstream>
#include <stdexcept>

namespace x360::xex {
namespace {
using Bytes=std::span<const uint8_t>;
void ensure(Bytes d, size_t off,size_t n, const char* what) {
    if(off>d.size()||n>d.size()-off) throw std::runtime_error(what);
}
uint32_t be32(Bytes d,size_t at){ensure(d,at,4,"Cabecalho XEX truncado");return
    (uint32_t(d[at])<<24)|(uint32_t(d[at+1])<<16)|(uint32_t(d[at+2])<<8)|d[at+3];}
uint16_t be16(Bytes d,size_t at){ensure(d,at,2,"Campo XEX truncado");return uint16_t(d[at]<<8|d[at+1]);}
uint16_t le16(Bytes d,size_t at){ensure(d,at,2,"Campo PE truncado");return uint16_t(d[at]|uint16_t(d[at+1])<<8);}
uint32_t le32(Bytes d,size_t at){ensure(d,at,4,"Campo PE truncado");return uint32_t(d[at])|uint32_t(d[at+1])<<8|uint32_t(d[at+2])<<16|uint32_t(d[at+3])<<24;}
uint32_t checked_add(uint32_t a,uint32_t b,const char* msg){if(uint64_t(a)+b>UINT32_MAX)throw std::runtime_error(msg);return a+b;}
std::string hex(uint32_t v){std::ostringstream s;s<<"0x"<<std::hex<<std::uppercase<<v;return s.str();}
std::optional<uint32_t> opt(const Headers& h,uint32_t k) {
    for (const auto& o:h.options) if(o.key==k) return o.value;
    return std::nullopt;
}
void parse_imports(Bytes d,uint32_t off, Headers& h) {
    if(off==0) return;
    ensure(d,off,12,"Import table XEX truncada");
    const uint32_t size=be32(d,off);
    if(size<12||size>2*1024*1024) throw std::runtime_error("Import table tamanho invalido");
    ensure(d,off,size,"Import table fora do XEX");
    const size_t end=off+size;
    uint32_t string_size=be32(d,off+4), count=be32(d,off+8);
    if(count>128 || string_size>size-12) throw std::runtime_error("Tabela de nomes/imports invalida");
    const size_t str_at=off+12;
    const size_t str_end=str_at+string_size;
    std::vector<std::string> names;
    // String table contains NUL-terminated strings and may include alignment.
    for(size_t pos=str_at;pos<str_end;){
        size_t stop=pos;
        while(stop<str_end && d[stop]!=0)++stop;
        if(stop==str_end)break;
        if(stop>pos && names.size()<128) names.emplace_back(reinterpret_cast<const char*>(d.data()+pos),stop-pos);
        pos=stop+1;
    }
    size_t p=(str_end+3u)&~size_t(3);
    for(uint32_t i=0;i<count;++i){
        if(p+40>end) throw std::runtime_error("Registro de importacao XEX truncado");
        uint32_t rec_size=be32(d,p);
        if(rec_size<40||rec_size>end-p) throw std::runtime_error("Tamanho de registro de imports invalido");
        uint16_t name_index=be16(d,p+36), n=be16(d,p+38);
        if(n>8192||40ull+uint64_t(n)*4>rec_size)throw std::runtime_error("Importacoes excessivas");
        ImportLibrary lib;
        lib.name=name_index<names.size()?names[name_index]:"biblioteca#"+std::to_string(name_index);
        lib.record_count=n;
        for(uint32_t j=0;j<n;++j) lib.guest_slots.push_back(be32(d,p+40+j*4));
        h.imports.push_back(std::move(lib));
        p=(p+rec_size+3u)&~size_t(3);
    }
}
}
Headers parse_headers(Bytes data){
    ensure(data,0,24,"Arquivo menor que o cabecalho XEX2");
    if(std::memcmp(data.data(),"XEX2",4)!=0)throw std::runtime_error("Assinatura XEX2 ausente");
    Headers h;
    h.module_flags=be32(data,4);
    h.pe_offset=be32(data,8);
    h.security_offset=be32(data,16);
    const uint32_t count=be32(data,20);
    if(count>4096||24ull+uint64_t(count)*8>data.size())throw std::runtime_error("Numero de optional headers invalido");
    if(h.pe_offset<24ull+uint64_t(count)*8 || h.pe_offset>=data.size()) throw std::runtime_error("Offset payload XEX invalido");
    for(uint32_t i=0;i<count;++i){
        const size_t p=24+size_t(i)*8;
        OptionalHeader o{be32(data,p),be32(data,p+4)};
        h.options.push_back(o);
        const uint8_t kind=static_cast<uint8_t>(o.key);
        if(kind==0||kind==1)continue;
        const size_t data_size=kind==0xFF ? 4u : size_t(kind)*4;
        ensure(data,o.value,data_size,"Optional header aponta fora do arquivo");
        if(kind==0xFF){
            const auto block_size=be32(data,o.value);
            if(block_size<4||block_size>16*1024*1024)throw std::runtime_error("Optional header com tamanho invalido");
            ensure(data,o.value,block_size,"Optional header estoura XEX");
        }
    }
    if(h.security_offset){
        ensure(data,h.security_offset,0x184,"Security Info XEX2 truncado");
        h.image_size=be32(data,h.security_offset+4);
        h.image_base=be32(data,h.security_offset+0x110);
        h.page_count=be32(data,h.security_offset+0x180);
        if(h.page_count>16384)throw std::runtime_error("Contagem de paginas XEX excessiva");
    }
    if(auto o=opt(h,0x00010201))h.image_base=*o;
    if(auto o=opt(h,0x00010100))h.entry_point=*o;
    if(auto o=opt(h,0x00040006)) {ensure(data,*o,0x18,"Execution Info invalida");h.title_id=be32(data,*o+0xC);}
    if(auto o=opt(h,0x000003FF)){
        h.file_format_offset=*o;
        ensure(data,*o,8,"File Format Info truncado");
        const uint32_t size=be32(data,*o);
        if(size<8||size>4*1024*1024)throw std::runtime_error("File Format Info tamanho invalido");
        ensure(data,*o,size,"File Format Info fora do XEX");
        h.file_format_size=size;
        h.encryption=be16(data,*o+4);
        h.compression=be16(data,*o+6);
        if(h.compression>3||h.encryption>1)throw std::runtime_error("Tipos XEX desconhecidos");
    }
    if(h.image_size>kMaxPeBytes)throw std::runtime_error("Image Size excede limite de 128 MiB");
    if(auto o=opt(h,0x000103FF))parse_imports(data,*o,h);
    return h;
}
std::string describe(const Headers& h){
    std::ostringstream s;
    s<<"XEX2 validado\nPayload: "<<hex(h.pe_offset)
     <<"\nSecurity Info: "<<hex(h.security_offset)
     <<"\nImage Base: "<<hex(h.image_base)
     <<"\nEntry Point: "<<hex(h.entry_point)
     <<"\nImage Size: "<<h.image_size
     <<"\nTitle ID: "<<hex(h.title_id)
     <<"\nEncryption: "<<h.encryption<<" (0=none,1=AES)"
     <<"\nCompression: "<<h.compression<<" (0=none,1=basic,2=LZX)"
     <<"\nOptional headers: "<<h.options.size()
     <<"\nImport libraries: "<<h.imports.size();
    for(const auto& lib:h.imports) s<<"\n  "<<lib.name<<" ("<<lib.record_count
        <<" registros de importacao; tipos/ordinais inspecionados apos mapeamento do PE)";
    return s.str();
}
std::array<uint8_t,16> derive_session_key(Bytes xex, const Headers& h,
                                           const std::array<uint8_t,16>& master_key) {
    if(!h.security_offset)throw std::runtime_error("XEX2 sem Security Info para obter Session Key");
    ensure(xex, uint64_t(h.security_offset)+0x150, 16,"Security Info: ImageKey truncada");
    const auto sealed=xex.subspan(uint64_t(h.security_offset)+0x150,16);
    // One AES-128 block with zero IV is AES-ECB decryption (no padding).
    // Only the caller can provide the external master key; do not embed keys.
    auto unwrapped=aes128_cbc_decrypt(sealed,master_key);
    std::array<uint8_t,16> session{};
    std::copy(unwrapped.begin(),unwrapped.end(),session.begin());
    std::fill(unwrapped.begin(),unwrapped.end(),0);
    return session;
}

std::vector<uint8_t> extract_pe(Bytes data, const Headers& h,
                                std::optional<std::array<uint8_t,16>> master_key) {
    if(h.pe_offset>=data.size())throw std::runtime_error("Payload XEX nao encontrado");
    Bytes payload=data.subspan(h.pe_offset);
    if(payload.size()>kMaxPeBytes)throw std::runtime_error("Payload maior que 128 MiB");
    std::vector<uint8_t> raw(payload.begin(),payload.end());
    if(h.encryption==1){
        if(!master_key)throw std::runtime_error("Imagem XEX criptografada: carregue um arquivo de chave externa de 16 bytes (nenhuma chave e incluida)");
        auto session=derive_session_key(data,h,*master_key);
        try { raw=aes128_cbc_decrypt(raw,session); }
        catch (...) {std::fill(session.begin(),session.end(),0);throw;}
        std::fill(session.begin(),session.end(),0);
    } else if(h.encryption!=0)throw std::runtime_error("Criptografia XEX nao suportada");
    if(h.compression==0)return raw;
    if(h.compression==2){
        if(h.file_format_size<36)throw std::runtime_error("XEX2 Normal: File Format Info requer 36 bytes");
        const size_t p=h.file_format_offset;
        const auto window_size=be32(data,p+8);
        const auto first_size=be32(data,p+12);
        std::array<uint8_t,20> first_hash{};
        ensure(data,p+16,20,"XEX2 Normal: SHA-1 inicial ausente");
        std::copy_n(data.begin()+static_cast<std::ptrdiff_t>(p+16),20,first_hash.begin());
        if(h.image_size==0)throw std::runtime_error("XEX2 Normal: Image Size ausente");
        return decompress_xex_lzx(raw,first_size,first_hash,window_size,h.image_size);
    }
    if(h.compression!=1)throw std::runtime_error("Compressao XEX desconhecida");
    if(h.file_format_size<16)throw std::runtime_error("File Format Info basic sem blocos");
    const size_t count=(h.file_format_size-8)/8;
    if((h.file_format_size-8)%8 || count>16384)throw std::runtime_error("Lista de blocos basic invalida");
    std::vector<uint8_t> out;
    size_t pos=0;
    Bytes headers=data;
    for(size_t i=0;i<count;++i){
        size_t at=static_cast<size_t>(h.file_format_offset)+8+i*8;
        uint32_t data_size=be32(headers,at),zero_size=be32(headers,at+4);
        if(data_size>raw.size()-pos)throw std::runtime_error("Bloco basic excede payload");
        if(uint64_t(out.size())+data_size+zero_size>kMaxPeBytes) throw std::runtime_error("Expansao basic excede limite");
        out.insert(out.end(),raw.begin()+static_cast<std::ptrdiff_t>(pos),raw.begin()+static_cast<std::ptrdiff_t>(pos+data_size));
        out.insert(out.end(),zero_size,0);
        pos+=data_size;
    }
    // AES-CBC payloads are block-aligned. The Basic descriptor describes the
    // meaningful compressed data; a final 0..15-byte all-zero suffix is padding,
    // not an extra Basic block. Reject nonzero trailing bytes and larger tails.
    if(pos!=raw.size()){
        const auto remaining=raw.size()-pos;
        if(remaining>=16 || !std::all_of(raw.begin()+static_cast<std::ptrdiff_t>(pos),
                                      raw.end(),[](uint8_t b){return b==0;}))
            throw std::runtime_error("Tamanho de blocos basic difere do payload");
    }
    return out;
}

bool PreparedImage::can_fetch(uint32_t at,size_t count) const {
    for(const auto& [low,high] : executable_ranges)
        if(at>=low && uint64_t(at)+count<=high)return true;
    return false;
}
PreparedImage map_pe(Bytes pe,const Headers& h, x360::xdvdfs::GuestRam& ram){
    ensure(pe,0,0x40,"PE muito pequeno");
    if(pe[0]!='M'||pe[1]!='Z')throw std::runtime_error("PE nao inicia por MZ: criptografia ou compressao nao resolvida");
    const uint32_t pe_offset=le32(pe,0x3C);
    ensure(pe,pe_offset,24,"Cabecalho NT PE truncado");
    if(std::memcmp(pe.data()+pe_offset,"PE\0\0",4)!=0)throw std::runtime_error("Assinatura PE nao encontrada");
    const uint16_t machine=le16(pe,pe_offset+4), nsec=le16(pe,pe_offset+6), optsize=le16(pe,pe_offset+20);
    if(machine!=0x01F2)throw std::runtime_error("PE nao e POWERPCBE 0x01F2");
    if(nsec==0||nsec>96||optsize<0x60)throw std::runtime_error("Secoes ou Optional Header PE invalidos");
    const size_t opt_offset=static_cast<size_t>(pe_offset)+24;
    ensure(pe,opt_offset,optsize,"Optional Header PE truncado");
    if(le16(pe,opt_offset)!=0x10B)throw std::runtime_error("Somente PE32 suportado");
    const uint32_t pe_entry=le32(pe,opt_offset+16), pe_base=le32(pe,opt_offset+28);
    const uint32_t size_of_image=le32(pe,opt_offset+56);
    const uint32_t size_of_headers=le32(pe,opt_offset+60);
    if(size_of_image==0||size_of_image>kMaxPeBytes)throw std::runtime_error("SizeOfImage PE fora do limite");
    const uint32_t base=h.image_base?h.image_base:pe_base;
    if(base < x360::xdvdfs::kGuestRamBase || uint64_t(base)+size_of_image > uint64_t(x360::xdvdfs::kGuestRamBase)+x360::xdvdfs::kRamSize)
        throw std::runtime_error("Image Base PE nao cabe na RAM emulada");
    const uint32_t entry=h.entry_point?h.entry_point:checked_add(base,pe_entry,"Entry Point overflow");
    if(entry<base || uint64_t(entry)>=uint64_t(base)+size_of_image)throw std::runtime_error("Entry Point fora da imagem PE");
    const size_t section_start=opt_offset+optsize;
    ensure(pe,section_start,size_t(nsec)*40,"Tabela de secoes PE truncada");
    PreparedImage result;
    result.base=base; result.pc=entry; result.image_size=size_of_image;result.imports=h.imports;
    // Always map headers first. BSS is implicitly zero in sparse GuestRam.
    const size_t headers_len=std::min<size_t>(size_of_headers,pe.size());
    if(headers_len>size_of_image)throw std::runtime_error("PE headers maiores que SizeOfImage");
    ram.write(base,pe.data(),headers_len);
    for(uint16_t i=0;i<nsec;++i){
        const size_t p=section_start+size_t(i)*40;
        PeSection sec;
        for(int j=0;j<8&&pe[p+j];++j)sec.name.push_back(char(pe[p+j]));
        sec.virtual_size=le32(pe,p+8);
        sec.virtual_address=le32(pe,p+12);
        sec.raw_size=le32(pe,p+16);
        sec.raw_address=le32(pe,p+20);
        sec.characteristics=le32(pe,p+36);
        if(uint64_t(sec.virtual_address)+std::max(sec.virtual_size,sec.raw_size)>size_of_image)
            throw std::runtime_error("Secao PE fora do SizeOfImage");
        if(sec.raw_size){
            ensure(pe,sec.raw_address,sec.raw_size,"Dados da secao PE fora do arquivo");
            ram.write(base+sec.virtual_address,pe.data()+sec.raw_address,sec.raw_size);
        }
        if(sec.characteristics & 0x20000000u){
            const uint32_t length=std::max(sec.virtual_size,sec.raw_size);
            if(length) result.executable_ranges.emplace_back(base+sec.virtual_address,base+sec.virtual_address+length);
        }
        result.sections.push_back(std::move(sec));
    }
    if(!result.can_fetch(entry,4))throw std::runtime_error("Entry Point nao pertence a secao executavel PE");
    return result;
}
} // namespace x360::xex