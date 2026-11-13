#include "xenos_pm4.hpp"
#include <array>
#include <cassert>
#include <iostream>
#include <vector>
using x360::gpu::CommandProcessor;
static uint32_t type3(uint32_t op, uint32_t n){return (3u<<30)|((n-1)<<16)|(op<<8);}
int main(){
    CommandProcessor p;
    std::string err;
    const std::array<uint32_t,3> one{(1u<<16)|0x10u,0x10203040u,0xAABBCCDDu};
    assert(p.submit(one,err));uint32_t value=0;
    assert(p.register_value(0x10,value)&&value==0x10203040u);
    assert(p.register_value(0x11,value)&&value==0xAABBCCDDu);
    const std::array<uint32_t,3> constant{type3(0x2d,2),(4u<<16)|0x5u,0xDEADBEEFu};
    assert(p.submit(constant,err));assert(p.register_value(0x2005,value)&&value==0xDEADBEEFu);
    const std::array<uint32_t,2> partial{type3(0x55,2),0x2000u};
    assert(p.submit(partial,err));assert(p.stats().packets==2);
    const std::array<uint32_t,1> rest{123u};assert(p.submit(rest,err));
    assert(p.register_value(0x2000,value)&&value==123u);
    const std::array<uint32_t,3> clear_test{type3(0x55,2),0xC000u,0x005522FFu};
    assert(p.submit(clear_test,err));
    assert(p.register_value(0xC000,value)&&value==0x005522FFu);
    // A register write alone must never draw a fake clear color.
    const std::array<uint32_t,2> event_write{type3(0x46,1),0x00000007u};
    assert(p.submit(event_write,err));assert(p.stats().event_writes==1);
    const std::array<uint32_t,3> draw{type3(0x22,2),1,2};assert(p.submit(draw,err));
    assert(p.stats().draws_seen==1 && p.stats().unsupported==1);
    const std::array<uint32_t,1> nop{2u<<30};assert(p.submit(nop,err));
    const std::array<uint32_t,2> nop3{type3(0x10,1),0u};
    assert(p.submit(nop3,err));assert(p.stats().nop_packets==2);
    const std::array<uint32_t,4> load{type3(0x2f,3),0x02000400u,0u,2u};
    const CommandProcessor::GuestWords reader=[](uint32_t address,size_t count,std::vector<uint32_t>& data){
        if(address!=0x02000400u||count!=2)return false;
        data={0x3F800000u,0x40000000u};return true;
    };
    assert(p.submit(load,err,reader));
    assert(p.stats().alu_loads==1);
    assert(p.register_value(0x4000,value)&&value==0x3F800000u);
    assert(p.register_value(0x4001,value)&&value==0x40000000u);
    // A source address without a mapped guest memory reader is not silently
    // interpreted as successful emulation.
    assert(p.submit(load,err));assert(p.stats().unsupported>=2);

    // Guest shader microcode is accepted as data, not treated as a GLSL program.
    const std::array<uint32_t,6> shader{type3(0x2B,5),0u,3u,
                                        0xAAAABBBB,0x12345678,0xCAFEBABEu};
    assert(p.submit(shader,err));
    assert(p.uploaded_shader_words(0)==3);
    assert(p.stats().shader_uploads==1);
    const std::array<uint32_t,3> shader_bad{type3(0x2B,2),2u,9u};
    assert(p.submit(shader_bad,err));
    assert(p.stats().shader_uploads==1);
    std::vector<uint32_t> too_long(CommandProcessor::kMaxQueuedWords+1);
    assert(!p.submit(too_long,err));
    std::cout<<"PM4 packet0/2/3, shader capture, guest register state, unsupported draw: PASS\n";
}