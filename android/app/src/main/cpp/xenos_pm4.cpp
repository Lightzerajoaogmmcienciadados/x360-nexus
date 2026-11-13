#include "xenos_pm4.hpp"
#include <algorithm>
#include <sstream>
#include <vector>

namespace x360::gpu {
namespace {
constexpr uint32_t kType0 = 0u;
constexpr uint32_t kType2 = 2u;
constexpr uint32_t kType3 = 3u;
constexpr uint32_t kSetConstant = 0x2du;
constexpr uint32_t kSetConstant2 = 0x55u;
constexpr uint32_t kDrawIndex = 0x22u;
constexpr uint32_t kDrawIndex2 = 0x36u;
constexpr uint32_t kRegRmw = 0x21u;
constexpr uint32_t kEventWrite = 0x46u;
constexpr uint32_t kLoadAlu = 0x2fu;
constexpr uint32_t kNop = 0x10u;
constexpr uint32_t kShaderConstants = 0x56u;
constexpr uint32_t kImLoad=0x27u;
constexpr uint32_t kImLoadImmediate=0x2bu;
}
CommandProcessor& processor() {static CommandProcessor instance; return instance;}
void CommandProcessor::reset(){
    std::lock_guard<std::mutex> lock(mutex_);
    queue_.clear();
    regs_.fill(0);
    shader_images_[0].clear();
    shader_images_[1].clear();
    stats_=Stats{};
}
void CommandProcessor::write_register(uint32_t index, uint32_t value) {
    if (index < regs_.size()) { regs_[index] = value; ++stats_.register_writes; }
    else { ++stats_.unsupported; stats_.last_event = "REG out of range"; }
}
bool CommandProcessor::submit(std::span<const uint32_t> words, std::string& error,
                              const GuestWords& read_guest) {
    if(words.size()>kMaxQueuedWords) {error="PM4: excessive submission";return false;}
    std::lock_guard<std::mutex> lock(mutex_);
    if(queue_.size()+words.size()>kMaxQueuedWords){error="PM4: queue full";return false;}
    queue_.insert(queue_.end(),words.begin(),words.end());
    consume_locked(read_guest); return true;
}
void CommandProcessor::consume_locked(const GuestWords& read_guest) {
    size_t consumed = 0;
    while(!queue_.empty() && consumed < kMaxQueuedWords) {
        const uint32_t header=queue_.front(), type=header>>30;
        const size_t count=type==kType2?0:size_t((header>>16)&0x3fffu)+1;
        // Treat malformed packet types as an error, not success.
        if(type!=kType0 && type!=kType2 && type!=kType3){
            queue_.pop_front();++stats_.unsupported;++stats_.packets;
            stats_.last_event="PM4 type 1 unsupported";++consumed;continue;
        }
        if(count > kMaxQueuedWords){++stats_.unsupported;stats_.last_event="PM4 payload too large";queue_.clear();break;}
        if(queue_.size() < count+1) break; // incomplete packet: keep queued until next submit
        queue_.pop_front();++consumed;
        std::vector<uint32_t> payload;
        payload.reserve(count);
        for(size_t i=0;i<count;++i){payload.push_back(queue_.front());queue_.pop_front();}
        consumed+=count; ++stats_.packets;
        if(type==kType2){++stats_.nop_packets;stats_.last_event="PM4 NOP type 2";continue;}
        if(type==kType0){
            const uint32_t first=header&0x7fffu;
            for(size_t i=0;i<count;++i)write_register(first+uint32_t(i),payload[i]);
            stats_.last_event="PM4 type 0 register write";
            continue;
        }
        const uint32_t opcode=(header>>8)&0x7fu;
        if(opcode==kNop) {
            ++stats_.nop_packets;
            stats_.last_event="PM4 type 3 NOP";
        }else if(opcode==kLoadAlu){
            // Xenos PM4_LOAD_ALU_CONSTANT: physical address, register/type,
            // count. Read the actual guest buffer via the caller's bounded
            // memory callback; do not synthesize constants on read failure.
            if(count!=3){++stats_.unsupported;stats_.last_event="LOAD_ALU_CONSTANT invalid packet length";continue;}
            const uint32_t address=payload[0]&0x3FFFFFFFu;
            const uint32_t offset_type=payload[1];
            const size_t words=payload[2]&0xFFFu;
            const uint32_t kind=(offset_type>>16)&0xFFu;
            uint32_t dest=offset_type&0x7FFu;
            switch(kind){
                case 0:dest+=0x4000;break;
                case 1:dest+=0x4800;break;
                case 2:dest+=0x4900;break;
                case 3:dest+=0x4908;break;
                case 4:dest+=0x2000;break;
                default:++stats_.unsupported;stats_.last_event="LOAD_ALU_CONSTANT unsupported register class";continue;
            }
            if(words==0 || words>256 || uint64_t(dest)+words>regs_.size()){
                ++stats_.unsupported;stats_.last_event="LOAD_ALU_CONSTANT exceeds bounded register range";continue;
            }
            std::vector<uint32_t> loaded;
            if(!read_guest || !read_guest(address,words,loaded) || loaded.size()!=words){
                ++stats_.unsupported;stats_.last_event="LOAD_ALU_CONSTANT: guest RAM not mapped/readable";continue;
            }
            for(size_t i=0;i<words;++i)write_register(dest+uint32_t(i),loaded[i]);
            ++stats_.alu_loads;
            stats_.last_event="PM4 LOAD_ALU_CONSTANT loaded from guest RAM";
        }else if(opcode==kImLoad || opcode==kImLoadImmediate){
            // Xbox Xenos shader uploads contain *guest microcode*, not GLSL.
            // Keep it available for the future translator, but never execute
            // arbitrary microcode on the host GPU without ISA translation.
            if(count<2){++stats_.unsupported;stats_.last_event="IM_LOAD short packet";continue;}
            const uint32_t stage=payload[0]&3u;
            const uint32_t start=payload[1]>>16;
            const uint32_t words=payload[1]&0xFFFFu;
            if(stage>=2||start!=0||words==0||words>4096){
                ++stats_.unsupported;stats_.last_event="IM_LOAD invalid stage/range";continue;
            }
            std::vector<uint32_t> shader;
            if(opcode==kImLoadImmediate){
                if(count<2u+words){
                    ++stats_.unsupported;stats_.last_event="IM_LOAD_IMMEDIATE truncated";continue;
                }
                shader.assign(payload.begin()+2,payload.begin()+2+words);
            }else{
                const uint32_t physical=payload[0]&~3u;
                if(!read_guest||!read_guest(physical,words,shader)||shader.size()!=words){
                    ++stats_.unsupported;stats_.last_event="IM_LOAD guest shader memory unavailable";continue;
                }
            }
            shader_images_[stage]=std::move(shader);
            ++stats_.shader_uploads;
            stats_.shader_dwords+=words;
            stats_.last_event="Xenos shader microcode captured (GLSL translation pending)";
        }else if(opcode==kShaderConstants && count>=1){
            const uint32_t dest=payload[0]&0xFFFFu;
            for(size_t i=1;i<count;++i)write_register(dest+uint32_t(i-1),payload[i]);
            stats_.last_event="PM4 SET_SHADER_CONSTANTS stored (microcode translation unavailable)";
        }else if(opcode==kSetConstant && count>=1){
            const uint32_t offset_type=payload[0];
            uint32_t start=offset_type&0x7ffu;
            switch((offset_type>>16)&0xffu){
                case 0:start+=0x4000;break; // ALU
                case 1:start+=0x4800;break; // FETCH
                case 2:start+=0x4900;break; // BOOL
                case 3:start+=0x4908;break; // LOOP
                case 4:start+=0x2000;break; // REGISTERS
                default:++stats_.unsupported;stats_.last_event="SET_CONSTANT unsupported type";continue;
            }
            for(size_t i=1;i<count;++i)write_register(start+uint32_t(i-1),payload[i]);
            stats_.last_event="PM4 SET_CONSTANT stored (no shader translation)";
        }else if(opcode==kSetConstant2 && count>=1){
            const uint32_t start=payload[0]&0xffffu;
            for(size_t i=1;i<count;++i)write_register(start+uint32_t(i-1),payload[i]);
            stats_.last_event="PM4 SET_CONSTANT2 stored";
        }else if(opcode==kRegRmw && count>=3){
            const uint32_t reg=payload[0]&0xffffu;
            if(reg>=regs_.size()){++stats_.unsupported;continue;}
            write_register(reg,(regs_[reg]&~payload[1])|payload[2]);
            stats_.last_event="PM4 REG_RMW applied";
        }else if(opcode==kEventWrite && count>=1){
            ++stats_.event_writes;
            stats_.last_event="PM4 EVENT_WRITE observed; VdSwap and EDRAM resolve not emulated";
        }else if(opcode==kDrawIndex || opcode==kDrawIndex2){
            ++stats_.draws_seen;++stats_.unsupported;
            stats_.last_event="PM4 DRAW_INDX queued: Xenos shaders/vertex fetch/EDRAM unavailable";
        }else{
            ++stats_.unsupported;
            std::ostringstream out;out<<"PM4 type3 opcode 0x"<<std::hex<<opcode<<" unimplemented";
            stats_.last_event=out.str();
        }
    }
}
size_t CommandProcessor::uploaded_shader_words(unsigned stage) const {
    std::lock_guard<std::mutex> lock(mutex_);
    return stage<shader_images_.size()?shader_images_[stage].size():0;
}
Stats CommandProcessor::stats() const {std::lock_guard<std::mutex> lock(mutex_);return stats_;}
bool CommandProcessor::register_value(uint32_t index,uint32_t& value) const {
    if(index>=regs_.size())return false;
    std::lock_guard<std::mutex> lock(mutex_);value=regs_[index];return true;
}
} // namespace x360::gpu