#include "xex_loader.hpp"
#include <array>
#include <cassert>
#include <cstdint>
#include <cstring>
#include <iostream>
#include <vector>
#include <stdexcept>

using namespace x360::xex;
namespace {
void u32be(std::vector<uint8_t>& v,size_t p,uint32_t x){v.at(p)=uint8_t(x>>24);v.at(p+1)=uint8_t(x>>16);v.at(p+2)=uint8_t(x>>8);v.at(p+3)=uint8_t(x);}
void u16be(std::vector<uint8_t>& v,size_t p,uint16_t x){v.at(p)=uint8_t(x>>8);v.at(p+1)=uint8_t(x);}
void u32le(std::vector<uint8_t>& v,size_t p,uint32_t x){for(int i=0;i<4;++i)v.at(p+i)=uint8_t(x>>(8*i));}
void u16le(std::vector<uint8_t>& v,size_t p,uint16_t x){v.at(p)=uint8_t(x);v.at(p+1)=uint8_t(x>>8);}
std::vector<uint8_t> pe_sample(){
  std::vector<uint8_t> p(0x400,0);
  p[0]='M';p[1]='Z';u32le(p,0x3c,0x80);
  std::memcpy(&p[0x80],"PE\0\0",4);
  u16le(p,0x84,0x1f2);u16le(p,0x86,1);u16le(p,0x94,0xe0);
  u16le(p,0x98,0x10b);u32le(p,0x98+16,0x1000);
  u32le(p,0x98+28,0x82000000);u32le(p,0x98+56,0x2000);u32le(p,0x98+60,0x200);
  std::memcpy(&p[0x178],".text",5);
  u32le(p,0x178+8,0x100);u32le(p,0x178+12,0x1000);
  u32le(p,0x178+16,16);u32le(p,0x178+20,0x200);u32le(p,0x178+36,0x60000020);
  const std::array<uint8_t,16> code{0x38,0x60,0,5,0x38,0x80,0,7,0x7c,0xa3,0x22,0x14,0x60,0,0,0};
  std::copy(code.begin(),code.end(),p.begin()+0x200);
  return p;
}
std::vector<uint8_t> xex_sample(const std::vector<uint8_t>& pe,uint16_t comp=0,uint16_t enc=0){
  std::vector<uint8_t> x(0x300+pe.size(),0);
  std::memcpy(x.data(),"XEX2",4);
  u32be(x,8,0x300);u32be(x,16,0x100);u32be(x,20,3);
  u32be(x,0x18,0x10201);u32be(x,0x1c,0x82000000);
  u32be(x,0x20,0x10100);u32be(x,0x24,0x82001000);
  u32be(x,0x28,0x3ff);u32be(x,0x2c,0x60);
  u32be(x,0x60,comp==1?16:8);u16be(x,0x64,enc);u16be(x,0x66,comp);
  if(comp==1){u32be(x,0x68,uint32_t(pe.size()));u32be(x,0x6c,0);}
  u32be(x,0x100+4,0x2000);u32be(x,0x100+0x110,0x82000000);
  std::copy(pe.begin(),pe.end(),x.begin()+0x300);
  return x;
}
}
int main(){
  const auto pe=pe_sample();
  auto xex=xex_sample(pe);
  auto headers=parse_headers(xex);
  assert(headers.image_base==0x82000000);
  assert(headers.entry_point==0x82001000);
  assert(headers.pe_offset==0x300);
  assert(headers.image_size==0x2000);
  auto payload=extract_pe(xex,headers);
  assert(payload==pe);
  x360::xdvdfs::GuestRam ram;
  auto prepared=map_pe(payload,headers,ram);
  assert(prepared.pc==0x82001000 && prepared.sections.size()==1);
  assert(prepared.can_fetch(0x82001000,16));
  assert(!prepared.can_fetch(0x82002000,4));
  uint8_t code[4];ram.read(0x82001000,code,4);
  assert(code[0]==0x38 && code[1]==0x60 && code[3]==5);

  auto basic=xex_sample(pe,1,0);
  assert(extract_pe(basic,parse_headers(basic))==pe);
  auto lzx=xex_sample(pe,2,0);
  bool blocked=false;
  try {extract_pe(lzx,parse_headers(lzx));}catch(const std::runtime_error& e){blocked=std::string(e.what()).find("LZX")!=std::string::npos;}
  assert(blocked);
  auto encrypted=xex_sample(pe,0,1);
  blocked=false;
  try {extract_pe(encrypted,parse_headers(encrypted));}catch(const std::runtime_error& e){blocked=std::string(e.what()).find("sessao")!=std::string::npos;}
  assert(blocked);
  std::vector<uint8_t> corrupt(24,0);blocked=false;
  try{parse_headers(corrupt);}catch(...){blocked=true;}assert(blocked);
  // AES-128 NIST ECB test vector, same as CBC's first block with zero IV.
  const std::array<uint8_t,16> k{0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15};
  const std::array<uint8_t,16> cipher{0x69,0xc4,0xe0,0xd8,0x6a,0x7b,4,0x30,0xd8,0xcd,0xb7,0x80,0x70,0xb4,0xc5,0x5a};
  auto plain=aes128_cbc_decrypt(cipher,k);
  for(int i=0;i<16;++i)assert(plain[i]==i*0x11);
  std::cout<<"XEX header, PE mapping, basic compression, AES and guards: OK\n";
}