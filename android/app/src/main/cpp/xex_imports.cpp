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
// Some homebrew-linked XEX2 import tables keep a prebuilt 16-byte
// li r11,ordinal; sc 2; nop; blr import trampoline in their text section.
// Accept it only if another catalog entry for the same library holds the
// exact type-0 ordinal; never infer from unreferenced code addresses.
std::optional<uint32_t> syscall_veneer_ordinal(const uint8_t bytes[16]) {
    const uint32_t a=word_be(bytes),b=word_be(bytes+4);
    const uint32_t c=word_be(bytes+8),d=word_be(bytes+12);
    if((a&0xFFFF0000u)!=0x39600000u || b!=0x44000002u ||
       c!=0x60000000u || d!=0x4E800020u) return {};
    const uint32_t ordinal=a&0xFFFFu;
    return ordinal?std::optional<uint32_t>(ordinal):std::nullopt;
}
std::optional<uint32_t> checked_veneer_ordinal(const uint8_t bytes[16]) {
    if(auto ordinal=veneer_ordinal(bytes))return ordinal;
    return syscall_veneer_ordinal(bytes);
}
// Indirect imported-function stub:
// lis r11,slot@ha; lwz r11,slot@l(r11); mtctr r11; bctr
// (or lis/ori to load a guest pointer). The target MUST be one of the
// type-0 slots from this XEX import library, or it is not a function thunk.
std::optional<uint32_t> veneer_target_slot(const uint8_t bytes[16]){
    const uint32_t a=word_be(bytes),b=word_be(bytes+4);
    const uint32_t c=word_be(bytes+8),d=word_be(bytes+12);
    if((a&0xFFFF0000u)!=0x3D600000u || c!=0x7D6903A6u ||
       (d!=0x4E800420u && d!=0x4E800421u))return {};
    const uint32_t upper=(a&0xFFFFu)<<16;
    if((b&0xFFFF0000u)==0x816B0000u) {
        return upper + static_cast<uint32_t>(static_cast<int32_t>(
            static_cast<int16_t>(b&0xFFFFu)));
    }
    if((b&0xFFFF0000u)==0x616B0000u)return upper|(b&0xFFFFu);
    return {};
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
        std::unordered_map<uint32_t,uint32_t> ordinal_by_variable_slot;
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
                ordinal_by_variable_slot.emplace(address,ordinal);
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
                    if(auto linked=checked_veneer_ordinal(code)) {
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
        // XEX2 sometimes lists an import's type-0 IAT slot but does not
        // enumerate its indirect PowerPC veneer as a second guest record.
        // Scan bounded executable ranges for *exact* four-instruction
        // indirect veneers pointing into an already verified type-0 slot.
        // Do not search for arbitrary branch targets or fabricate 142 thunks.
        constexpr uint64_t kMaxScanPerLibrary=32ull*1024*1024;
        constexpr uint64_t kScanChunk=16*1024;
        uint64_t budget=kMaxScanPerLibrary;
        if(!ordinal_by_variable_slot.empty()){
            for(const auto& [from,to]:executable_ranges){
                uint64_t lo=std::max<uint64_t>(from,base);
                uint64_t hi=std::min<uint64_t>(to,end);
                lo=(lo+3)&~uint64_t(3);
                if(hi<=lo+15 || budget<16)continue;
                const uint64_t scan_bytes=std::min<uint64_t>(hi-lo,budget);
                hi=lo+scan_bytes;
                budget-=scan_bytes;result.scanned_executable_bytes+=scan_bytes;
                for(uint64_t chunk=lo;chunk+16<=hi;chunk+=kScanChunk){
                    const size_t size=static_cast<size_t>(std::min<uint64_t>(kScanChunk+12,hi-chunk));
                    std::vector<uint8_t> bytes(size);
                    try{ram.read(static_cast<uint32_t>(chunk),bytes.data(),size);}
                    catch(...){++result.malformed;continue;}
                    const uint64_t stop=std::min<uint64_t>(kScanChunk,hi-chunk);
                    for(size_t offset=0;offset<stop && offset+16<=size;offset+=4){
                        auto imported_slot=veneer_target_slot(bytes.data()+offset);
                        if(!imported_slot)continue;
                        auto match=ordinal_by_variable_slot.find(*imported_slot);
                        if(match==ordinal_by_variable_slot.end())continue;
                        const uint32_t address=static_cast<uint32_t>(chunk+offset);
                        const uint64_t key=(uint64_t(module)<<32)|address;
                        if(!seen.insert(key).second)continue;
                        result.thunks.push_back({address,match->second,module,true,*imported_slot});
                        ++result.scanned_veneers;
                    }
                }
            }
        }
    }
    return result;
}
size_t patch_import_thunks(const ImportInventory& inv,
    const std::vector<std::pair<uint32_t,uint32_t>>& executable_ranges,
    x360::xdvdfs::GuestRam& ram) {
    static constexpr std::array<uint8_t,16> patch{
        0x44,0x00,0x00,0x02, // sc 2; HLE dispatch keyed by PC and ordinal
        0x4E,0x80,0x00,0x20, // blr (handler returns directly to validated LR)
        0x60,0x00,0x00,0x00, // nop
        0x60,0x00,0x00,0x00, // nop
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
            if(t.linked_variable_slot!=0){
                const auto slot=veneer_target_slot(code);
                if(!slot || *slot!=t.linked_variable_slot)continue;
                uint8_t original[4]{};ram.read(*slot,original,4);
                const uint32_t raw=word_be(original);
                if((raw>>24)!=0 || (raw&0xFFFFu)!=t.ordinal)continue;
            }else{
                const auto ordinal=checked_veneer_ordinal(code);
                if(!ordinal || *ordinal!=t.ordinal)continue;
            }
        }else if((word>>24)!=1 || (word&0xffff)!=t.ordinal)continue;
        ram.write(t.address,patch.data(),patch.size());
        ++patched;
    }
    return patched;
}

std::string ImportInventory::describe() const {
    std::ostringstream o;
    o<<"IMPORT_THUNKS="<<thunks.size()<<"; IMPORTS: registros="<<records<<", variaveis="<<variables
      <<", variaveis validadas="<<variables_found.size()<<", thunks identificados="<<thunks.size()<<", malformados="<<malformed
      <<", zerados/nao inicializados="<<uninitialized
      <<", candidatos de veneer="<<executable_candidates<<", veneers ligados por ordinal="<<paired_veneers
      <<", veneers vinculados a IAT="<<scanned_veneers<<", bytes .text inspecionados="<<scanned_executable_bytes
      <<", tipos desconhecidos="<<unrecognized_types
      <<", bibliotecas nao mapeadas="<<unsupported_libraries
      <<"\nHLE: somente ordinais implementados podem ser chamados. Os demais causam pausa detalhada.";
    for(size_t i=0;i<thunks.size() && i<16;++i){
        const auto& t=thunks[i];
        o<<"\n  "<<(t.module==1?"xboxkrnl.exe":"xam.xex")
          <<" ordinal="<<t.ordinal<<" thunk=0x"<<std::hex<<std::uppercase<<t.address<<std::dec
          <<(t.linked_variable_slot?" (veneer ligado a slot IAT verificado)":(
              t.inferred_from_veneer?" (veneer+ordinal validado)":" (descriptor type=1)"));
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