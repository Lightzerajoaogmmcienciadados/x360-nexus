#include "xdvdfs.hpp"
#include <array>
#include <cassert>
#define CHECK(expr) do { if (!(expr)) throw std::runtime_error(std::string("CHECK failed: ") + #expr); } while(0)
#include <cstdio>
#include <cstring>
#include <exception>
#include <fcntl.h>
#include <iostream>
#include <stdexcept>
#include <string>
#include <unistd.h>
#include <vector>

using namespace x360::xdvdfs;
namespace {
struct TempFile {
    int fd=-1;
    TempFile() {
        char path[]="/tmp/xdvdfs-test-XXXXXX";
        fd=mkstemp(path);
        if(fd<0) throw std::runtime_error("mkstemp failed");
        unlink(path);
    }
    ~TempFile(){if(fd>=0)close(fd);}
    TempFile(const TempFile&)=delete;
    TempFile& operator=(const TempFile&)=delete;
};
void put(int fd,uint64_t addr,const void* data,size_t n) {
    const auto written=pwrite(fd,data,n,static_cast<off_t>(addr));
    if(written!=static_cast<ssize_t>(n)) throw std::runtime_error("pwrite failed");
}
void le16(uint8_t* b,uint16_t n){b[0]=static_cast<uint8_t>(n);b[1]=static_cast<uint8_t>(n>>8);}
void le32(uint8_t* b,uint32_t n){for(int i=0;i<4;i++)b[i]=static_cast<uint8_t>(n>>(8*i));}
constexpr char magic[]="MICROSOFT*XBOX*MEDIA";
// Build small but structurally real, fully synthetic XDVDFS image.
void make_disc(int fd,uint64_t base=0, bool cycle=false,bool bad_file=false) {
    uint64_t size=base+64*2048;
    if(ftruncate(fd,static_cast<off_t>(size))!=0) throw std::runtime_error("ftruncate failed");
    std::array<uint8_t,2048> volume{};
    std::memcpy(volume.data(),magic,20);
    std::memcpy(volume.data()+2028,magic,20);
    le32(volume.data()+20,33); le32(volume.data()+24,2048);
    put(fd,base+32*2048,volume.data(),volume.size());
    std::array<uint8_t,2048> root{};
    // Root BST node at 0: default.xex; right node at 64: assets/.
    le16(root.data()+2,cycle ? 0 : 16); // right offset in DWORDs
    le32(root.data()+4,bad_file ? 9999999u : 40u);
    le32(root.data()+8,16);
    root[12]=0x20;
    root[13]=11;
    std::memcpy(root.data()+14,"default.xex",11);
    if(!cycle) {
        le32(root.data()+64+4,42);
        le32(root.data()+64+8,2048);
        root[64+12]=0x10;
        root[64+13]=6;
        std::memcpy(root.data()+64+14,"assets",6);
    } else {
        le16(root.data(),1); // left points to itself? offset 4 -> zeroed invalid entry
        le16(root.data()+2,1); // test duplicate references -> cycle detection
    }
    put(fd,base+33*2048,root.data(),root.size());
    std::array<uint8_t,2048> assets{};
    le32(assets.data()+4,44);
    le32(assets.data()+8,3);
    assets[12]=0x20;
    assets[13]=8;
    std::memcpy(assets.data()+14,"icon.bin",8);
    put(fd,base+42*2048,assets.data(),assets.size());
    const std::array<uint8_t,16> xex={
        'X','E','X','2',0,0,0,0,
        0x38,0x60,0,5,0x38,0x80,0,7
    };
    put(fd,base+40*2048,xex.data(),xex.size());
    const char icon[]="PNG";
    put(fd,base+44*2048,icon,sizeof(icon)-1);
}
void check_mount_and_load(uint64_t base,const std::string& layout) {
    TempFile disc;
    make_disc(disc.fd,base);
    FdReader reader(disc.fd);
    Image image(reader);
    CHECK(image.volume().layout==layout);
    CHECK(image.entries().size()==3);
    const Entry* f=image.find_default_xex();
    CHECK(f && f->size==16);
    CHECK(image.entries().at(2).path=="assets/icon.bin");
    GuestRam guest;
    const LoadResult result=load_xex(image,*f,guest);
    CHECK(result.guest_address==kGuestRamBase);
    CHECK(result.loaded_bytes==16);
    CHECK(result.xex2_signature);
    CHECK(guest.allocated_bytes()==4096);
    std::array<uint8_t,16> raw{};
    guest.read(kGuestRamBase,raw.data(),raw.size());
    CHECK(std::memcmp(raw.data(),"XEX2",4)==0);
    CHECK(raw[11]==5 && raw[15]==7);
    TempFile extracted;
    const auto bytes=copy_file_to_fd(image,*f,extracted.fd);
    CHECK(bytes==16);
    std::array<uint8_t,16> cmp{};
    CHECK(pread(extracted.fd,cmp.data(),cmp.size(),0)==16);
    CHECK(cmp==raw);
}
void invalid_disc() {
    TempFile f;
    CHECK(ftruncate(f.fd,131072)==0);
    FdReader r(f.fd);
    bool rejected=false;
    try {Image img(r);}catch(const std::exception&){rejected=true;}
    CHECK(rejected);
}
void bad_directory() {
    TempFile f;
    make_disc(f.fd,0,true,false);
    FdReader r(f.fd);
    bool rejected=false;
    try {Image img(r);}catch(const std::exception&){rejected=true;}
    CHECK(rejected);
}
void bad_file_offset() {
    TempFile f;
    make_disc(f.fd,0,false,true);
    FdReader r(f.fd);
    bool rejected=false;
    try {Image img(r);}catch(const std::exception&){rejected=true;}
    CHECK(rejected);
}
void memory_bounds() {
    GuestRam ram;
    std::array<uint8_t,16> zero{};
    ram.read(kGuestRamBase,zero.data(),zero.size());
    CHECK((zero==std::array<uint8_t,16>{}));
    bool denied=false;
    try { ram.write(0xA2000000u,zero.data(),zero.size()); }
    catch(const std::exception&){denied=true;}
    CHECK(denied);
}
}
int main(){
    try {
        check_mount_and_load(0,"XISO");
        std::cout << "PASS: XISO root + nested list + RAM staging + extraction\n";
        check_mount_and_load(0x0FD90000ull,"XGD2");
        std::cout << "PASS: XGD2 ISO offset\n";
        check_mount_and_load(0x02080000ull,"XGD3");
        std::cout << "PASS: XGD3 ISO offset\n";
        invalid_disc();std::cout << "PASS: invalid magic denied\n";
        bad_directory();std::cout << "PASS: cyclic tree denied\n";
        bad_file_offset();std::cout << "PASS: out of bounds file denied\n";
        memory_bounds();std::cout << "PASS: 512 MiB guest RAM boundaries\n";
        return 0;
    } catch(const std::exception& e){std::cerr << "FAIL: " <<e.what()<<"\n";return 1;}
}
