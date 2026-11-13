#include "xex_loader.hpp"
#include <array>
#ifdef NDEBUG
#undef NDEBUG
#endif
#include <cassert>
#include <cstdint>
#include <cstring>
#include <iostream>
#include <vector>
#include <stdexcept>
#include <algorithm>

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
struct LzxBitWriter {
  std::vector<uint8_t> out;
  uint16_t word=0;
  unsigned used=0;
  void put(uint32_t value,unsigned count) {
    for(unsigned i=count;i>0;--i){
      word=uint16_t((word<<1)|((value>>(i-1))&1));
      if(++used==16){out.push_back(uint8_t(word));out.push_back(uint8_t(word>>8));used=0;word=0;}
    }
  }
  void align(){if(used){word=uint16_t(word<<(16-used));out.push_back(uint8_t(word));out.push_back(uint8_t(word>>8));used=0;word=0;}}
};
std::vector<uint8_t> lzx_uncompressed(const std::vector<uint8_t>& plain) {
  LzxBitWriter b;
  b.put(0,1); // CAB LZX intel transform off
  b.put(3,3); // uncompressed block
  b.put(uint32_t(plain.size())>>8,16);
  b.put(uint32_t(plain.size())&0xFF,8);
  b.align();
  // R0/R1/R2 = 1 (little endian uint32)
  for(int n=0;n<3;++n){b.out.insert(b.out.end(),{1,0,0,0});}
  b.out.insert(b.out.end(),plain.begin(),plain.end());
  return b.out;
}
void lzx_pretree(LzxBitWriter& b) {
  for(unsigned symbol=0;symbol<20;++symbol)
    b.put(symbol==18?1u:(symbol==16||symbol==17?2u:0u),4);
}
void lzx_zeros(LzxBitWriter& b,unsigned count) {
  while(count>=20){
    unsigned take=std::min(count,51u);
    b.put(0,1); // pretree 18, canonical code 0
    b.put(take-20,5);
    count-=take;
  }
  if(count){assert(count>=4 && count<=19);b.put(3,2);b.put(count-4,4);} // 17 -> 11
}
std::vector<uint8_t> lzx_literals_fixture(bool aligned) {
  LzxBitWriter b;
  b.put(0,1); // Intel E8 translation disabled
  b.put(aligned?2:1,3);
  b.put(0,16);b.put(4,8); // four literal output bytes
  if(aligned){
    for(unsigned i=0;i<8;++i)b.put((i<2)?1u:0u,3);
  }
  lzx_pretree(b);
  lzx_zeros(b,77); // main tree codes for 'M' and 'Z'
  b.put(2,2); // pretree 16 -> 10, delta 16: length 1
  lzx_zeros(b,12);
  b.put(2,2); // 'Z'
  lzx_zeros(b,165);
  lzx_pretree(b);
  lzx_zeros(b,240); // remaining match symbols all zero
  lzx_pretree(b);
  lzx_zeros(b,249); // length tree all zero (all literals)
  b.put(0,1);b.put(1,1);b.put(0,1);b.put(1,1); // M Z M Z
  b.align();
  return b.out;
}
std::vector<uint8_t> xex_normal_from_stream(const std::vector<uint8_t>& raw,
                                             uint32_t output_size){
  if(raw.size()>65535)throw std::runtime_error("test chunk too large");
  std::vector<uint8_t> block(24,0);
  block.push_back(uint8_t(raw.size()>>8));block.push_back(uint8_t(raw.size()));
  block.insert(block.end(),raw.begin(),raw.end());
  block.push_back(0);block.push_back(0);
  auto x=xex_sample(std::vector<uint8_t>(output_size),2,0);
  x.resize(0x300);
  u32be(x,0x60,36);
  u32be(x,0x68,0x8000);
  u32be(x,0x6C,uint32_t(block.size()));
  auto hash=sha1_lzx_block(block);
  std::copy(hash.begin(),hash.end(),x.begin()+0x70);
  u32be(x,0x104,output_size);
  x.insert(x.end(),block.begin(),block.end());
  return x;
}
std::vector<uint8_t> xex_normal_fixture(const std::vector<uint8_t>& plain);
std::vector<uint8_t> xex_normal_fixture(const std::vector<uint8_t>& plain){
  auto raw=lzx_uncompressed(plain);
  if(raw.size()>65535)throw std::runtime_error("test chunk too large");
  std::vector<uint8_t> block(24,0); // next block size = 0; hash of next block zeroed
  block.push_back(uint8_t(raw.size()>>8));block.push_back(uint8_t(raw.size()));
  block.insert(block.end(),raw.begin(),raw.end());
  block.push_back(0);block.push_back(0); // zero chunk terminates this block
  auto x=xex_sample(plain,2,0);
  x.resize(0x300);
  u32be(x,0x60,36);
  u32be(x,0x68,0x8000); // LZX window 32 KiB
  u32be(x,0x6C,uint32_t(block.size()));
  auto hash=sha1_lzx_block(block);
  std::copy(hash.begin(),hash.end(),x.begin()+0x70);
  u32be(x,0x104,uint32_t(plain.size())); // decompressed file length
  x.insert(x.end(),block.begin(),block.end());
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

  // Two string-table names with per-string 4-byte padding, followed by two
  // library records. string_table.count counts names, not import libraries.
  {
    auto with_imports=xex_sample(pe);
    u32be(with_imports,0x14,4); // four optional XEX headers
    u32be(with_imports,0x30,0x000103FF);
    u32be(with_imports,0x34,0x80);
    const size_t off=0x80;
    u32be(with_imports,off,12+24+44+44);
    u32be(with_imports,off+4,24);
    u32be(with_imports,off+8,2);
    std::memcpy(with_imports.data()+off+12,"xboxkrnl.exe",12);
    std::memcpy(with_imports.data()+off+12+16,"xam.xex",7);
    const size_t r0=off+12+24,r1=r0+44;
    for(const auto& record : std::vector<std::pair<size_t,uint16_t>>{{r0,0},{r1,1}}){
      u32be(with_imports,record.first,44);
      u16be(with_imports,record.first+36,record.second);
      u16be(with_imports,record.first+38,1);
    }
    u32be(with_imports,r0+40,0x82001000);
    u32be(with_imports,r1+40,0x82001100);
    auto hdr=parse_headers(with_imports);
    assert(hdr.imports.size()==2);
    assert(hdr.imports[0].name=="xboxkrnl.exe");
    assert(hdr.imports[1].name=="xam.xex");
    assert(hdr.imports[0].guest_slots[0]==0x82001000);
    assert(hdr.imports[1].guest_slots[0]==0x82001100);
  }
  // Xenia lays out import records at exactly (optional_header + 12 +
  // string_table.size), not at the next 4-byte aligned absolute offset.
  // A non-aligned string table used to shift the library header and silently
  // produce wrong import slot addresses in the current XEX loader.
  {
    auto with_imports=xex_sample(pe);
    u32be(with_imports,0x14,4);
    u32be(with_imports,0x30,0x000103FF);
    u32be(with_imports,0x34,0x80);
    const size_t off=0x80;
    constexpr uint32_t table_bytes=13; // "xboxkrnl.exe" + NUL, no trailing pad
    u32be(with_imports,off,12+table_bytes+44);
    u32be(with_imports,off+4,table_bytes);
    u32be(with_imports,off+8,1);
    std::memcpy(with_imports.data()+off+12,"xboxkrnl.exe",12);
    const size_t record=off+12+table_bytes; // unaligned on purpose
    u32be(with_imports,record,44);
    u16be(with_imports,record+36,0);
    u16be(with_imports,record+38,1);
    u32be(with_imports,record+40,0x82001010);
    const auto hdr=parse_headers(with_imports);
    assert(hdr.imports.size()==1);
    assert(hdr.imports[0].name=="xboxkrnl.exe");
    assert(hdr.imports[0].guest_slots.size()==1);
    assert(hdr.imports[0].guest_slots[0]==0x82001010);
    // A truncated last name must not be accepted as an import library.
    u32be(with_imports,off+4,12); // missing terminating NUL
    bool rejected=false;
    try{(void)parse_headers(with_imports);}catch(const std::runtime_error&){rejected=true;}
    assert(rejected);
  }
  auto basic=xex_sample(pe,1,0);
  assert(extract_pe(basic,parse_headers(basic))==pe);
  const auto lzx=xex_normal_fixture(pe);
  assert(extract_pe(lzx,parse_headers(lzx))==pe);
  for(const bool aligned : {false,true}) {
    auto compressed=xex_normal_from_stream(lzx_literals_fixture(aligned),4);
    const auto expanded=extract_pe(compressed,parse_headers(compressed));
    assert(expanded==std::vector<uint8_t>({'M','Z','M','Z'}));
  }
  auto damaged=lzx;
  damaged.back()^=0x80;
  bool blocked=false;
  try{(void)extract_pe(damaged,parse_headers(damaged));}catch(const std::runtime_error& e){blocked=std::string(e.what()).find("SHA-1")!=std::string::npos;}
  assert(blocked);
  auto encrypted=xex_sample(pe,0,1);
  blocked=false;
  try {extract_pe(encrypted,parse_headers(encrypted));}catch(const std::runtime_error& e){blocked=std::string(e.what()).find("chave externa")!=std::string::npos;}
  assert(blocked);
  std::vector<uint8_t> corrupt(24,0);blocked=false;
  try{parse_headers(corrupt);}catch(...){blocked=true;}assert(blocked);
  // AES-128 NIST ECB test vector, same as CBC's first block with zero IV.
  const std::array<uint8_t,16> k{0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15};
  const std::array<uint8_t,16> cipher{0x69,0xc4,0xe0,0xd8,0x6a,0x7b,4,0x30,0xd8,0xcd,0xb7,0x80,0x70,0xb4,0xc5,0x5a};
  auto plain=aes128_cbc_decrypt(cipher,k);
  for(int i=0;i<16;++i)assert(plain[i]==i*0x11);
  // Derive the session key using the *synthetic* AES-128 NIST vector.
  auto synthetic_key_xex=xex_sample(pe,0,1);
  std::copy(cipher.begin(),cipher.end(),synthetic_key_xex.begin()+0x100+0x150);
  auto session=derive_session_key(synthetic_key_xex,parse_headers(synthetic_key_xex),k);
  for(int i=0;i<16;++i)assert(session[i]==i*0x11);
  const std::array<uint8_t,20> expected_sha1{0xa9,0x99,0x3e,0x36,0x47,0x06,0x81,0x6a,0xba,0x3e,
         0x25,0x71,0x78,0x50,0xc2,0x6c,0x9c,0xd0,0xd8,0x9d};
  assert(sha1_lzx_block(std::span<const uint8_t>(reinterpret_cast<const uint8_t*>("abc"),3)) == expected_sha1);
  // Basic descriptor + CBC block padding: zero suffix up to 15 bytes only.
  {
    auto padded=xex_sample(pe,1,0);
    padded.insert(padded.end(),8,0);
    assert(extract_pe(padded,parse_headers(padded))==pe);
    padded.back()=0x41;
    bool padding_rejected=false;
    try{(void)extract_pe(padded,parse_headers(padded));}
    catch(const std::runtime_error&){padding_rejected=true;}
    assert(padding_rejected);
  }
  // Import record entries point to guest virtual addresses, not ordinals.
  // The word at that address encodes type (0=variable,1=thunk) and ordinal.
  {
    x360::xdvdfs::GuestRam imported_ram;
    constexpr uint32_t base=0x82000000;
    const std::array<uint8_t,4> thunk_word{1,0,1,0x52}; // ordinal 0x152=338, KeTlsAlloc
    const std::array<uint8_t,4> variable_word{0,0,0,4};
    imported_ram.write(base+0x1000,thunk_word.data(),thunk_word.size());
    imported_ram.write(base+0x1020,variable_word.data(),variable_word.size());
    ImportLibrary lib;
    lib.name="XBOXKRNL.EXE";
    lib.record_count=3;
    lib.guest_slots={base+0x1000,base+0x1020,base+0x2004}; // last outside image
    const auto inv=inspect_import_thunks({lib},base,0x2000,imported_ram);
    assert(inv.records==3 && inv.variables==1 && inv.malformed==1);
    assert(inv.thunks.size()==1);
    assert(inv.thunks[0].address==base+0x1000);
    assert(inv.thunks[0].ordinal==338 && inv.thunks[0].module==1);
    assert(inv.describe().find("ordinal=338")!=std::string::npos);
    // Only valid verified type-1 stubs in executable ranges may be patched.
    const auto count=patch_import_thunks(inv,{{base+0x1000,base+0x1010}},imported_ram);
    assert(count==1);
    uint8_t patched[16]{};
    imported_ram.read(base+0x1000,patched,16);
    assert(patched[0]==0x44 && patched[1]==0 && patched[2]==0 && patched[3]==2);
    imported_ram.read(base+0x1020,patched,4);
    assert(patched[0]==0 && patched[3]==4); // variable never patched
  }
  {
    x360::xdvdfs::GuestRam ram;
    constexpr uint32_t base=0x82000000;
    const std::array<uint8_t,4> unknown{0x7c,0x63,0x22,0x14};
    ram.write(base+0x1040,unknown.data(),4);
    ImportLibrary lib;lib.name="xam.xex";lib.guest_slots={base+0x1040};
    const auto inv=inspect_import_thunks({lib},base,0x2000,ram);
    assert(inv.unrecognized_types==1 && inv.malformed==0 && inv.thunks.empty());
    assert(inv.describe().find("UNKNOWN")!=std::string::npos);
    assert(patch_import_thunks(inv,{{base+0x1040,base+0x1050}},ram)==0);
  }
  {
    x360::xdvdfs::GuestRam ram;
    constexpr uint32_t base=0x82000000;
    ImportLibrary lib;lib.name="xboxkrnl.exe";lib.guest_slots={base+0x1000};
    const auto inv=inspect_import_thunks({lib},base,0x2000,ram);
    assert(inv.uninitialized==1 && inv.malformed==0 && inv.thunks.empty());
    assert(inv.describe().find("nao inicializados=1")!=std::string::npos);
  }
  // v5.6: A whole checked PowerPC veneer can be associated with a type-0
  // ordinal by the embedded low immediate, when the XEX slot itself contains
  // code (not an intact type-1 descriptor). Other PPC code must never match.
  {
    x360::xdvdfs::GuestRam ram;
    constexpr uint32_t base=0x82000000;
    const uint8_t veneer[16]{
        0x3D,0x60,0,0,     // lis r11,0
        0x61,0x6B,1,0x52,  // ori r11,r11,338
        0x7D,0x69,0x03,0xA6, // mtctr r11
        0x4E,0x80,0x04,0x20  // bctr
    };
    const uint8_t data[4]{0,0,1,0x52};
    ram.write(base+0x1000,veneer,sizeof(veneer));
    ram.write(base+0x2000,data,sizeof(data));
    ImportLibrary lib;lib.name="xboxkrnl.exe";
    lib.guest_slots={base+0x1000,base+0x2000};
    const auto inv=inspect_import_thunks({lib},base,0x3000,ram,
                                          {{base+0x1000,base+0x1010}});
    assert(inv.records==2 && inv.variables==1 && inv.thunks.size()==1);
    assert(inv.executable_candidates==1 && inv.paired_veneers==1);
    assert(inv.thunks[0].inferred_from_veneer);
    assert(inv.thunks[0].ordinal==338);
    assert(patch_import_thunks(inv,{{base+0x1000,base+0x1010}},ram)==1);
    uint8_t patched[4]{};ram.read(base+0x1000,patched,4);
    assert(patched[0]==0x44 && patched[3]==2);
    // A similar veneer with an unlisted ordinal must not be patched.
    x360::xdvdfs::GuestRam bad;
    const uint8_t different[16]{0x3D,0x60,0,0,0x61,0x6B,3,0xE7,
                                 0x7D,0x69,0x03,0xA6,0x4E,0x80,0x04,0x20};
    bad.write(base+0x1000,different,16);bad.write(base+0x2000,data,4);
    const auto inv_bad=inspect_import_thunks({lib},base,0x3000,bad,
                                          {{base+0x1000,base+0x1010}});
    assert(inv_bad.thunks.empty() && inv_bad.paired_veneers==0);
  }
  // v5.6 synthetic regression: the required 154 type-0 plus 142 type-1
  // imports must be counted from actual descriptors, never a hard-coded
  // Minecraft title-specific count.
  {
    x360::xdvdfs::GuestRam ram;
    constexpr uint32_t base=0x82000000;
    ImportLibrary lib;lib.name="xboxkrnl.exe";lib.name_index=0;
    for(uint32_t i=0;i<154;++i){
      uint32_t ordinal=0x100+i;
      const uint8_t var[4]{0,0,uint8_t(ordinal>>8),uint8_t(ordinal)};
      const uint32_t var_slot=base+0x4000+i*4;
      ram.write(var_slot,var,sizeof(var));lib.guest_slots.push_back(var_slot);
      if(i<142){
        const uint8_t thunk[4]{1,0,uint8_t(ordinal>>8),uint8_t(ordinal)};
        const uint32_t thunk_addr=base+0x1000+i*16;
        ram.write(thunk_addr,thunk,sizeof(thunk));lib.guest_slots.push_back(thunk_addr);
      }
    }
    const auto inventory=inspect_import_thunks({lib},base,0x6000,ram,
                                                {{base+0x1000,base+0x2000}});
    assert(inventory.variables_found.size()==154);
    assert(inventory.thunks.size()==142);
    assert(patch_import_thunks(inventory,{{base+0x1000,base+0x2000}},ram)==142);
    uint8_t result[8]{};
    ram.read(base+0x1000,result,sizeof(result));
    assert(result[0]==0x44 && result[3]==0x02); // sc 2
    assert(result[4]==0x4E && result[7]==0x20); // blr
  }
  // Explicitly cataloged PPC syscall veneer can also be recovered.
  {
    x360::xdvdfs::GuestRam ram;
    constexpr uint32_t base=0x82000000;
    const uint8_t veneer[16]{0x39,0x60,1,0x91, 0x44,0,0,2,
                              0x60,0,0,0, 0x4E,0x80,0,0x20};
    const uint8_t value[4]{0,0,1,0x91};
    ram.write(base+0x1000,veneer,16);
    ram.write(base+0x2000,value,4);
    ImportLibrary lib;lib.name="xam.xex";
    lib.guest_slots={base+0x2000,base+0x1000};
    const auto inv=inspect_import_thunks({lib},base,0x3000,ram,
                                          {{base+0x1000,base+0x1010}});
    assert(inv.thunks.size()==1 && inv.thunks[0].ordinal==0x191);
    assert(patch_import_thunks(inv,{{base+0x1000,base+0x1010}},ram)==1);
  }
  // v5.7: some executables expose only type-0 IAT records. Discover
  // *actual* stubs in .text through their full indirect branch into IAT.
  {
    x360::xdvdfs::GuestRam ram;
    constexpr uint32_t base=0x82000000;
    ImportLibrary lib;lib.name="xboxkrnl.exe";
    for(uint32_t i=0;i<142;++i){
      const uint32_t ordinal=338+(i%100);
      const uint32_t slot=base+0x4000+i*4;
      const uint32_t address=base+0x1000+i*16;
      const uint32_t target=0x816B0000u|(slot&0xFFFFu); // lwz r11,slot@l(r11)
      const uint32_t ins[4]{0x3D608200u,target,0x7D6903A6u,0x4E800420u};
      for(int j=0;j<4;++j){
        const uint32_t be=ins[j];
        const uint8_t word[4]{uint8_t(be>>24),uint8_t(be>>16),uint8_t(be>>8),uint8_t(be)};
        ram.write(address+j*4,word,4);
      }
      const uint8_t descriptor[4]{0,0,uint8_t(ordinal>>8),uint8_t(ordinal)};
      ram.write(slot,descriptor,4);
      lib.guest_slots.push_back(slot); // no type-1 record in table
    }
    const auto inv=inspect_import_thunks({lib},base,0x6000,ram,
                                          {{base+0x1000,base+0x2000}});
    assert(inv.variables_found.size()==142);
    assert(inv.scanned_veneers==142);
    assert(inv.thunks.size()==142);
    assert(patch_import_thunks(inv,{{base+0x1000,base+0x2000}},ram)==142);
    uint8_t stub[8]{};
    ram.read(base+0x1000,stub,8);
    assert(stub[0]==0x44 && stub[3]==2 && stub[4]==0x4E && stub[7]==0x20);
  }
  std::cout<<"XEX/PE, LZX uncompressed frame, SHA-1 chain, AES key unwrap: OK\n";
}