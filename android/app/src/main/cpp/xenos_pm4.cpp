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
}
CommandProcessor& processor() {static CommandProcessor instance; return instance;}
void CommandProcessor::write_register(uint32_t index, uint32_t value) {
    if (index < regs_.size()) { regs_[index] = value; ++stats_.register_writes; }
    else { ++stats_.unsupported; stats_.last_event = "REG out of range"; }
}
bool CommandProcessor::submit(std::span<const uint32_t> words, std::string& error) {
    if(words.size()>kMaxQueuedWords) {error="PM4: excessive submission";return false;}
    std::lock_guard<std::mutex> lock(mutex_);
    if(queue_.size()+words.size()>kMaxQueuedWords){error="PM4: queue full";return false;}
    queue_.insert(queue_.end(),words.begin(),words.end());
    consume_locked(); return true;
}
void CommandProcessor::consume_locked() {
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
        if(type==kType2){stats_.last_event="PM4 NOP type 2";continue;}
        if(type==kType0){
            const uint32_t first=header&0x7fffu;
            for(size_t i=0;i<count;++i)write_register(first+uint32_t(i),payload[i]);
            stats_.last_event="PM4 type 0 register write";
            continue;
        }
        const uint32_t opcode=(header>>8)&0x7fu;
        if(opcode==kSetConstant && count>=1){
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
        }else if(opcode==kDrawIndex || opcode==kDrawIndex2){
            ++stats_.draws_seen;++stats_.unsupported;
            stats_.last_event="PM4 DRAW_INDX seen: guest shader/index translation unavailable";
        }else{
            ++stats_.unsupported;
            std::ostringstream out;out<<"PM4 type3 opcode 0x"<<std::hex<<opcode<<" unimplemented";
            stats_.last_event=out.str();
        }
    }
}
Stats CommandProcessor::stats() const {std::lock_guard<std::mutex> lock(mutex_);return stats_;}
bool CommandProcessor::register_value(uint32_t index,uint32_t& value) const {
    if(index>=regs_.size())return false;
    std::lock_guard<std::mutex> lock(mutex_);value=regs_[index];return true;
}
} // namespace x360::gpu