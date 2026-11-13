#include "xex_loader.hpp"
#include <algorithm>
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
            if(!ordinal){++result.malformed;continue;}
            if(type==0) {++result.variables;continue;}
            if(type!=1 || uint64_t(address)+16>end){++result.malformed;continue;}
            const uint64_t key=(uint64_t(module)<<32)|address;
            if(!seen.insert(key).second){++result.malformed;continue;}
            result.thunks.push_back({address,ordinal,module});
        }
    }
    return result;
}
std::string ImportInventory::describe() const {
    std::ostringstream o;
    o<<"IMPORTS: registros="<<records<<", variaveis="<<variables
      <<", thunks identificados="<<thunks.size()<<", malformados="<<malformed
      <<", bibliotecas nao mapeadas="<<unsupported_libraries
      <<"\nHLE: somente ordinais implementados podem ser chamados. Os demais causam pausa detalhada.";
    for(size_t i=0;i<thunks.size() && i<16;++i){
        const auto& t=thunks[i];
        o<<"\n  "<<(t.module==1?"xboxkrnl.exe":"xam.xex")
          <<" ordinal="<<t.ordinal<<" thunk=0x"<<std::hex<<std::uppercase<<t.address<<std::dec;
    }
    if(thunks.size()>16)o<<"\n  ... "<<(thunks.size()-16)<<" thunks adicionais";
    return o.str();
}
} // namespace x360::xex