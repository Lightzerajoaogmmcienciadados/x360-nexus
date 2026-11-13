#include "xex_loader.hpp"
#include <algorithm>
#include <array>
#include <cctype>
#include <iomanip>
#include <sstream>
#include <unordered_set>
#include <unordered_map>
#include <optional>

namespace x360::xex {
namespace {
uint32_t word_be(const uint8_t* b) {
    return (uint32_t(b[0])<<24)|(uint32_t(b[1])<<16)|(uint32_t(b[2])<<8)|b[3];
}
bool exec_contains(uint32_t address, size_t length,
    const std::vector<std::pair<uint32_t,uint32_t>>& ranges) {
    for (const auto& range:ranges) {
        if(address>=range.first && uint64_t(address)+length<=range.second)return true;
    }
    return false;
}
// A checked PowerPC indirect-branch import veneer, never an arbitrary .text
// instruction. Also require its embedded low immediate to match an existing
// type-0 descriptor ordinal before inferring a type-1 counterpart.
std::optional<uint32_t> veneer_ordinal(const uint8_t bytes[16]) {
    const auto a=word_be(bytes),b=word_be(bytes+4);
    const auto c=word_be(bytes+8),d=word_be(bytes+12);
    // lis r11,imm16 (addis r11,r0,..), ori r11,r11,imm16
    if((a&0xFFFF0000)!=0x3D600000 || (b&0xFFFF0000)!=0x616B0000)return {};
    // mtspr CTR,r11; bctr/bctrl. SPR encoding is split across two fields.
    const uint32_t xo=(c>>1)&1023, spr=((c>>16)&31)|(((c>>11)&31)<<5);
    if((c>>26)!=31 || ((c>>21)&31)!=11 || xo!=467 || spr!=9)return {};
    if(d!=0x4E800420 && d!=0x4E800421)return {};
    return b&0xFFFF;
}
} // namespace

ImportInventory inspect_import_thunks(const std::vector<ImportLibrary>& libs,
    uint32_t base, uint32_t image_size, const x360::xdvdfs::GuestRam& ram,
    const std::vector<std::pair<uint32_t,uint32_t>>& executable_ranges) {
    ImportInventory result;
    const uint64_t end=uint64_t(base)+image_size;
    std::unordered_set<uint64_t> seen;
    for(const auto& lib:libs) {
        std::string name=lib.name;
        std::transform(name.begin(),name.end(),name.begin(),[](unsigned char c){return char(std::tolower(c));});
        const uint32_t module=name=="xboxkrnl.exe"?1u:name=="xam.xex"?2u:0u;
        if(!module){++result.unsupported_libraries;continue;}
        struct Candidate{uint32_t address, ordinal;};
        std::vector<Candidate> possible_veneers;
        std::unordered_set<uint32_t> value_ordinals;
        for(auto address:lib.guest_slots) {
            ++result.records;
            if(address%4!=0 || address<base || uint64_t(address)+4>end) {
                ++result.malformed;continue;
            }
            uint8_t bytes[4]{};
            try {ram.read(address,bytes,sizeof(bytes));}
            catch(...){++result.malformed;continue;}
            const uint32_t value=word_be(bytes);
            const uint32_t type=value>>24;
            const uint32_t ordinal=value&0xFFFFu;
            if(value==0){
                ++result.uninitialized;
                if(result.examples.size()<8){
                    std::ostringstream sample;sample<<name<<" slot=0x"<<std::hex<<std::uppercase
                        <<address<<" uninitialized (zero import descriptor)";
                    result.examples.push_back(sample.str());
                }
                continue;
            }
            if(type==0 && ordinal!=0) {
                const uint64_t key=(uint64_t(module)<<32)|address;
                if(!seen.insert(key).second){++result.malformed;continue;}
                ++result.variables;
                value_ordinals.insert(ordinal);
                result.variables_found.push_back({address,ordinal,module});
                continue;
            }
            if(type==1 && ordinal!=0){
                if(uint64_t(address)+16>end){++result.malformed;continue;}
                const uint64_t key=(uint64_t(module)<<32)|address;
                if(!seen.insert(key).second){++result.malformed;continue;}
                result.thunks.push_back({address,ordinal,module,false});
                continue;
            }
            // Some XEXs contain executable code at a record address rather
            // than an intact type-1 descriptor. Do not infer an ordinal from
            // an arbitrary PPC instruction; identify only a complete known
            // veneer and match its ordinal against a type-0 import in *this*
            // library. No guessed patching to reach an expected thunk count.
            bool candidate=false;
            if(!executable_ranges.empty() && uint64_t(address)+16<=end &&
               exec_contains(address,16,executable_ranges)) {
                uint8_t code[16]{};
                try {ram.read(address,code,sizeof(code));
                    if(auto linked=veneer_ordinal(code)) {
                        ++result.executable_candidates;
                        possible_veneers.push_back({address,*linked});candidate=true;
                    }
                } catch(...) { /* no candidate */ }
            }
            if(!candidate) {
                ++result.unrecognized_types;
                if(result.examples.size()<12){
                    std::ostringstream sample;sample<<name<<" slot=0x"<<std::hex<<std::uppercase
                        <<address<<" word=0x"<<std::setw(8)<<std::setfill('0')<<value
                        <<" class=unverified (not auto-patched)";
                    result.examples.push_back(sample.str());
                }
            }
        }
        for(const auto& c:possible_veneers){
            if(!value_ordinals.contains(c.ordinal)){
                ++result.unrecognized_types;
                continue;
            }
            const uint64_t key=(uint64_t(module)<<32)|c.address;
            if(!seen.insert(key).second){++result.malformed;continue;}
            result.thunks.push_back({c.address,c.ordinal,module,true});
            ++result.paired_veneers;
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
        if(t.inferred_from_veneer){
            uint8_t code[16]{};ram.read(t.address,code,16);
            const auto ordinal=veneer_ordinal(code);
            if(!ordinal || *ordinal!=t.ordinal)continue;
        }else if((word>>24)!=1 || (word&0xffff)!=t.ordinal)continue;
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
      <<", candidatos de veneer="<<executable_candidates<<", veneers ligados por ordinal="<<paired_veneers
      <<", tipos desconhecidos="<<unrecognized_types
      <<", bibliotecas nao mapeadas="<<unsupported_libraries
      <<"\nHLE: somente ordinais implementados podem ser chamados. Os demais causam pausa detalhada.";
    for(size_t i=0;i<thunks.size() && i<16;++i){
        const auto& t=thunks[i];
        o<<"\n  "<<(t.module==1?"xboxkrnl.exe":"xam.xex")
          <<" ordinal="<<t.ordinal<<" thunk=0x"<<std::hex<<std::uppercase<<t.address<<std::dec
          <<(t.inferred_from_veneer?" (veneer+ordinal validado)":" (descriptor type=1)");
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