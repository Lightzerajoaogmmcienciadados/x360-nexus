#include "xex_loader.hpp"
#include <algorithm>
#include <array>
#include <cctype>
#include <iomanip>
#include <sstream>
#include <unordered_set>

namespace x360::xex {
ImportInventory inspect_import_thunks(const std::vector<ImportLibrary>& libs,
    uint32_t base, uint32_t image_size, const x360::xdvdfs::GuestRam& ram) {
    ImportInventory result;
    const uint64_t end=uint64_t(base)+image_size;
    std::unordered_set<uint64_t> seen;
    for(const auto& lib:libs) {
        std::string name=lib.name;
        std::transform(name.begin(),name.end(),name.begin(),[](unsigned char c){return char(std::tolower(c));});
        const uint32_t module=name=="xboxkrnl.exe"?1u:name=="xam.xex"?2u:0u;
        if(!module){++result.unsupported_libraries;continue;}
        for(auto address:lib.guest_slots) {
            ++result.records;
            if(address%4!=0 || address<base || uint64_t(address)+4>end) {
                ++result.malformed;continue;
            }
            uint8_t bytes[4]{};
            try {ram.read(address,bytes,sizeof(bytes));}
            catch(...){++result.malformed;continue;}
            const uint32_t value=(uint32_t(bytes[0])<<24)|(uint32_t(bytes[1])<<16)|
                (uint32_t(bytes[2])<<8)|uint32_t(bytes[3]);
            const uint32_t type=value>>24;
            const uint32_t ordinal=value&0xFFFFu;
            if(value==0){
                ++result.uninitialized;
                if(result.examples.size()<8){
                    std::ostringstream sample;
                    sample<<name<<" slot=0x"<<std::hex<<std::uppercase<<address
                          <<" uninitialized (zero import descriptor)";
                    result.examples.push_back(sample.str());
                }
                continue;
            }
            if(!ordinal){++result.malformed;continue;}
            if(type==0) {
                const uint64_t key=(uint64_t(module)<<32)|address;
                if(!seen.insert(key).second){++result.malformed;continue;}
                ++result.variables;
                result.variables_found.push_back({address,ordinal,module});
                continue;
            }
            if(type!=1){
                ++result.unrecognized_types;
                if(result.examples.size()<8){
                    std::ostringstream sample;
                    sample<<name<<" slot=0x"<<std::hex<<std::uppercase<<address
                          <<" value=0x"<<std::setw(8)<<std::setfill('0')<<value
                          <<" high_byte=0x"<<type;
                    result.examples.push_back(sample.str());
                }
                continue;
            }
            if(uint64_t(address)+16>end){++result.malformed;continue;}
            const uint64_t key=(uint64_t(module)<<32)|address;
            if(!seen.insert(key).second){++result.malformed;continue;}
            result.thunks.push_back({address,ordinal,module});
        }
    }
    return result;
}
size_t patch_import_thunks(const ImportInventory& inv,
    const std::vector<std::pair<uint32_t,uint32_t>>& executable_ranges,
    x360::xdvdfs::GuestRam& ram) {
    static constexpr std::array<uint8_t,16> patch{
        0x44,0x00,0x00,0x02, // sc 2 (Rust intercepts only catalogued thunk PCs)
        0x60,0x00,0x00,0x00, // nop
        0x60,0x00,0x00,0x00, // nop
        0x4E,0x80,0x00,0x20, // blr
    };
    size_t patched=0;
    for(const auto& t:inv.thunks) {
        bool exec=false;
        for(const auto& range:executable_ranges) {
            if(t.address>=range.first && uint64_t(t.address)+patch.size()<=range.second) {
                exec=true;break;
            }
        }
        // Never turn data entries into code and never create thunks out of
        // unknown descriptors merely to satisfy an import count.
        if(!exec)continue;
        uint8_t descriptor[4]{};
        ram.read(t.address,descriptor,4);
        const uint32_t word=uint32_t(descriptor[0])<<24|uint32_t(descriptor[1])<<16|
            uint32_t(descriptor[2])<<8|descriptor[3];
        if((word>>24)!=1 || (word&0xffff)!=t.ordinal)continue;
        ram.write(t.address,patch.data(),patch.size());
        ++patched;
    }
    return patched;
}

std::string ImportInventory::describe() const {
    std::ostringstream o;
    o<<"IMPORTS: registros="<<records<<", variaveis="<<variables
      <<", variaveis validadas="<<variables_found.size()<<", thunks identificados="<<thunks.size()<<", malformados="<<malformed
      <<", zerados/nao inicializados="<<uninitialized
      <<", tipos desconhecidos="<<unrecognized_types
      <<", bibliotecas nao mapeadas="<<unsupported_libraries
      <<"\nHLE: somente ordinais implementados podem ser chamados. Os demais causam pausa detalhada.";
    for(size_t i=0;i<thunks.size() && i<16;++i){
        const auto& t=thunks[i];
        o<<"\n  "<<(t.module==1?"xboxkrnl.exe":"xam.xex")
          <<" ordinal="<<t.ordinal<<" thunk=0x"<<std::hex<<std::uppercase<<t.address<<std::dec;
    }
    if(thunks.size()>16)o<<"\n  ... "<<(thunks.size()-16)<<" thunks adicionais";
    for(size_t i=0;i<variables_found.size() && i<8;++i){
        const auto& var=variables_found[i];
        o<<"\n  [VARIABLE] "<<(var.module==1?"xboxkrnl.exe":"xam.xex")
         <<" ordinal="<<var.ordinal<<" slot=0x"<<std::hex<<std::uppercase<<var.address<<std::dec;
    }
    for(const auto& sample:examples)o<<"\n  [UNKNOWN] "<<sample;
    return o.str();
}
} // namespace x360::xex