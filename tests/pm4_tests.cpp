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
    const std::array<uint32_t,3> draw{type3(0x22,2),1,2};assert(p.submit(draw,err));
    assert(p.stats().draws_seen==1 && p.stats().unsupported==1);
    const std::array<uint32_t,1> nop{2u<<30};assert(p.submit(nop,err));
    std::vector<uint32_t> too_long(CommandProcessor::kMaxQueuedWords+1);
    assert(!p.submit(too_long,err));
    std::cout<<"PM4 packet0/2/3, ring fragment, unsupported draw: PASS\n";
}