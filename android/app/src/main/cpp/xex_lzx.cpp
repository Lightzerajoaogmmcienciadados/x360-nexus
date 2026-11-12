// Independently written LZX (CAB-style) decoder for educational XEX2 parsing.
// No console keys or third-party source code are bundled here.
#include "xex_loader.hpp"
#include <algorithm>
#include <array>
#include <cstdint>
#include <cstring>
#include <stdexcept>
#include <string>
#include <vector>

namespace x360::xex {
namespace {
using Bytes = std::span<const uint8_t>;
[[noreturn]] void bad(const char* msg) { throw std::runtime_error(msg); }
void check(Bytes b, size_t p, size_t n) {
    if (p > b.size() || n > b.size() - p) bad("LZX: dados truncados");
}
uint16_t get_be16(Bytes b, size_t p) {
    check(b,p,2); return (uint16_t(b[p]) << 8) | b[p+1];
}
uint32_t get_be32(Bytes b,size_t p) {
    check(b,p,4); return (uint32_t(b[p]) << 24) | (uint32_t(b[p+1]) << 16) |
       (uint32_t(b[p+2]) << 8) | b[p+3];
}
uint32_t rol(uint32_t x, unsigned n) { return (x << n) | (x >> (32 - n)); }

// SHA-1 is only used to verify integrity of the XEX LZX block chain.
std::array<uint8_t,20> digest(Bytes bytes) {
    std::array<uint32_t,5> h{0x67452301,0xefcdab89,0x98badcfe,0x10325476,0xc3d2e1f0};
    const uint64_t bit_length=uint64_t(bytes.size())*8;
    size_t padded=bytes.size()+1;
    if (padded > kMaxXexBytes+128) bad("LZX: SHA-1 entrada muito grande");
    while ((padded%64)!=56) ++padded;
    std::vector<uint8_t> data(padded+8);
    std::copy(bytes.begin(),bytes.end(),data.begin());
    data[bytes.size()]=0x80;
    for (unsigned i=0;i<8;++i) data[padded+i]=uint8_t(bit_length >> (56-8*i));
    for(size_t off=0;off<data.size();off+=64) {
        uint32_t w[80]{};
        for(unsigned i=0;i<16;++i) w[i]=get_be32(data,off+i*4);
        for(unsigned i=16;i<80;++i) w[i]=rol(w[i-3]^w[i-8]^w[i-14]^w[i-16],1);
        auto a=h[0],b=h[1],c=h[2],d=h[3],e=h[4];
        for(unsigned i=0;i<80;++i) {
            uint32_t f=0,k=0;
            if(i<20) {f=(b&c)|(~b&d);k=0x5a827999;}
            else if(i<40) {f=b^c^d;k=0x6ed9eba1;}
            else if(i<60) {f=(b&c)|(b&d)|(c&d);k=0x8f1bbcdc;}
            else {f=b^c^d;k=0xca62c1d6;}
            auto t=rol(a,5)+f+e+k+w[i];e=d;d=c;c=rol(b,30);b=a;a=t;
        }
        h[0]+=a;h[1]+=b;h[2]+=c;h[3]+=d;h[4]+=e;
    }
    std::array<uint8_t,20> result{};
    for(unsigned i=0;i<5;++i)for(unsigned j=0;j<4;++j)result[i*4+j]=uint8_t(h[i]>>(24-j*8));
    return result;
}

struct Bits {
    Bytes input; size_t pos=0; uint64_t buffer=0; unsigned available=0;
    explicit Bits(Bytes b):input(b){}
    uint32_t read(unsigned n) {
        if(n>24)bad("LZX: solicitacao excessiva de bits");
        if(!n) return 0;
        while(available<n){
            check(input,pos,2);
            const uint16_t word=uint16_t(input[pos])|uint16_t(input[pos+1])<<8;
            pos+=2; buffer=(buffer<<16)|word;available+=16;
        }
        const uint32_t val=uint32_t((buffer>>(available-n)) & ((uint64_t(1)<<n)-1));
        available-=n;
        if(available)buffer&=(uint64_t(1)<<available)-1;else buffer=0;
        return val;
    }
    void align_word() { buffer=0; available=0; }
    uint32_t raw32(){check(input,pos,4);auto v=uint32_t(input[pos])|
        uint32_t(input[pos+1])<<8|uint32_t(input[pos+2])<<16|uint32_t(input[pos+3])<<24;
        pos+=4;return v;}
    Bytes raw(size_t n){check(input,pos,n);auto ret=input.subspan(pos,n);pos+=n;return ret;}
    void skip(size_t n){(void)raw(n);}
};

// Canonical Huffman code decoder; invalid prefix and length tables are rejected.
class Huffman {
    struct Node { int next[2]{-1,-1}; int symbol=-1; };
    std::vector<Node> nodes_;
public:
    void build(const std::vector<uint8_t>& lengths, bool allow_empty=false) {
        nodes_.clear();nodes_.emplace_back();
        std::array<uint32_t,17> counts{},code{};
        unsigned nonzero=0;
        for(const auto len:lengths){if(len>16)bad("LZX: comprimento Huffman invalido");
            if(len){++counts[len];++nonzero;}}
        if(!nonzero){if(allow_empty)return;bad("LZX: Huffman sem simbolos");}
        uint32_t next=0;
        for(unsigned len=1;len<=16;++len){ next=(next+counts[len-1])*2;
            if(next+counts[len]>(1u<<len))bad("LZX: codigos Huffman sobrepostos");
            code[len]=next; }
        for(size_t sym=0;sym<lengths.size();++sym){
            const unsigned len=lengths[sym];if(!len)continue;
            const uint32_t word=code[len]++;
            int node=0;
            for(unsigned depth=0;depth<len;++depth){
                const int b=int((word>>(len-depth-1))&1);
                int child=nodes_[node].next[b];
                if(child<0){child=int(nodes_.size());nodes_[node].next[b]=child;nodes_.emplace_back();}
                node=child;
                if(depth+1!=len && nodes_[node].symbol!=-1)bad("LZX: arvore Huffman invalida");
            }
            if(nodes_[node].symbol!=-1||nodes_[node].next[0]!=-1||nodes_[node].next[1]!=-1)
                bad("LZX: codigo Huffman duplicado");
            nodes_[node].symbol=int(sym);
        }
    }
    int decode(Bits& bits)const {
        int node=0;
        for(unsigned i=0;i<=16;++i){
            if(nodes_[node].symbol>=0)return nodes_[node].symbol;
            if(i==16)break;
            node=nodes_[node].next[bits.read(1)];
            if(node<0)bad("LZX: sequencia Huffman invalida");
        }
        bad("LZX: codigo Huffman longo demais");
    }
};

void read_lengths(Bits& bits,std::vector<uint8_t>& lens,size_t begin,size_t end) {
    if(end>lens.size()||begin>end)bad("LZX: intervalo de codigos invalido");
    std::vector<uint8_t> pre(20);
    for(auto& n:pre)n=uint8_t(bits.read(4));
    Huffman tree;tree.build(pre);
    for(size_t i=begin;i<end;){
        int value=tree.decode(bits);
        if(value==17||value==18||value==19){
            unsigned copies=0;
            if(value==17)copies=bits.read(4)+4;
            else if(value==18)copies=bits.read(5)+20;
            else copies=bits.read(1)+4;
            if(copies>end-i)bad("LZX: repeticao excedeu tabela Huffman");
            uint8_t decoded=0;
            if(value==19){decoded=uint8_t((unsigned(lens[i])+17u-unsigned(tree.decode(bits)))%17u);}
            std::fill_n(lens.begin()+static_cast<std::ptrdiff_t>(i),copies,decoded);
            i+=copies;
        } else {
            lens[i]=uint8_t((unsigned(lens[i])+17u-unsigned(value))%17u);
            ++i;
        }
    }
}

std::vector<uint8_t> decode_lzx(Bytes stream,size_t total,int window_bits) {
    if(window_bits<15||window_bits>21)bad("LZX: janela invalida");
    if(total==0||total>kMaxPeBytes)bad("LZX: tamanho de expansao invalido");
    const size_t window=size_t(1)<<window_bits;
    // LZX position-slot counts for windows of 32 KiB .. 2 MiB.
    static constexpr int slots[]{30,32,34,36,38,42,50};
    const int slot_count=slots[window_bits-15];
    std::vector<uint32_t> position_base(slot_count);
    std::vector<unsigned> extra(slot_count);
    for(int i=0;i<slot_count;++i){
        extra[i]=i<4?0u:std::min<unsigned>(17,unsigned(i/2-1));
        if(i)position_base[i]=position_base[i-1]+(1u<<extra[i-1]);
    }
    std::vector<uint8_t> main_lengths(256+slot_count*8),length_lengths(249),aligned_lengths(8);
    Huffman main_tree,length_tree,aligned_tree;
    Bits bits(stream);
    const bool has_e8=bits.read(1)!=0;
    uint32_t intel_size=0;
    if(has_e8)intel_size=(bits.read(16)<<16)|bits.read(16);
    uint32_t r0=1,r1=1,r2=1;
    uint32_t block_remaining=0,block_size=0;
    uint32_t last_block_size=0;
    int type=0, previous_type=0;
    std::vector<uint8_t> out;
    out.reserve(std::min(total,size_t(8*1024*1024)));
    while(out.size()<total){
        if(block_remaining==0){
            if(previous_type==3 && (last_block_size&1u)) bits.skip(1);
            type=int(bits.read(3));
            if(type<1||type>3)bad("LZX: tipo de bloco invalido");
            block_size=(bits.read(16)<<8)|bits.read(8);
            if(!block_size)bad("LZX: bloco de tamanho zero");
            block_remaining=block_size;
            if(type==2){
                for(auto& n:aligned_lengths)n=uint8_t(bits.read(3));
                aligned_tree.build(aligned_lengths);
            }
            if(type==1||type==2){
                read_lengths(bits,main_lengths,0,256);
                read_lengths(bits,main_lengths,256,main_lengths.size());
                main_tree.build(main_lengths);
                read_lengths(bits,length_lengths,0,length_lengths.size());
                length_tree.build(length_lengths,true);
            } else {
                bits.align_word();
                r0=bits.raw32();r1=bits.raw32();r2=bits.raw32();
                if(!r0||!r1||!r2)bad("LZX: estado de offsets invalido");
            }
            previous_type=type;
            last_block_size=block_size;
        }
        const size_t quota=std::min<size_t>(block_remaining,total-out.size());
        if(type==3){
            auto raw=bits.raw(quota);out.insert(out.end(),raw.begin(),raw.end());
            block_remaining-=static_cast<uint32_t>(quota);
            continue;
        }
        size_t produced=0;
        while(produced<quota){
            int sym=main_tree.decode(bits);
            if(sym<256){out.push_back(uint8_t(sym));++produced;continue;}
            sym-=256;
            const unsigned slot=unsigned(sym>>3);
            uint32_t length=uint32_t(sym&7);
            if(length==7)length+=uint32_t(length_tree.decode(bits));
            length+=2;
            if(length>quota-produced)bad("LZX: match excedeu limite do bloco/frame");
            if(slot>=unsigned(slot_count))bad("LZX: slot de offset invalido");
            uint32_t offset=0;
            if(slot==0){offset=r0;}
            else if(slot==1){offset=r1;r1=r0;r0=offset;}
            else if(slot==2){offset=r2;r2=r0;r0=offset;}
            else {
                if(slot==3)offset=1;
                else {
                    offset=position_base[slot]-2;
                    const unsigned n=extra[slot];
                    if(type==2 && n>=3){
                        if(n>3)offset+=bits.read(n-3)<<3;
                        offset+=uint32_t(aligned_tree.decode(bits));
                    } else offset+=bits.read(n);
                }
                r2=r1;r1=r0;r0=offset;
            }
            if(offset==0||offset>window||offset>out.size())bad("LZX: referencia de historico invalida");
            for(uint32_t j=0;j<length;++j)out.push_back(out[out.size()-offset]);
            produced+=length;
        }
        block_remaining-=static_cast<uint32_t>(produced);
    }
    // Intel E8 translation is relevant to x86 CAB content, not PowerPC executables;
    // refuse transformed streams rather than silently emit incorrect PE instructions.
    if(has_e8 && intel_size!=0)bad("LZX: fluxo com Intel E8 nao suportado para PE PowerPC");
    return out;
}
} // namespace

std::array<uint8_t,20> sha1_lzx_block(std::span<const uint8_t> data){return digest(data);}

std::vector<uint8_t> unpack_xex_lzx_blocks(Bytes input, uint32_t initial_size,
                                             std::array<uint8_t,20> initial_hash) {
    if(initial_size<26 || initial_size>input.size())bad("LZX: tamanho do primeiro bloco invalido");
    size_t cursor=0;
    uint32_t block_size=initial_size;
    std::vector<uint8_t> stream;
    unsigned chain=0;
    while(block_size){
        if(++chain>32768)bad("LZX: blocos demais");
        check(input,cursor,block_size);
        const auto block=input.subspan(cursor,block_size);
        if(digest(block)!=initial_hash)bad("LZX: hash SHA-1 do bloco nao confere");
        const uint32_t next_size=get_be32(block,0);
        std::array<uint8_t,20> next_hash{};
        std::copy_n(block.begin()+4,20,next_hash.begin());
        size_t pos=24;
        bool terminated=false;
        while(pos+2<=block.size()){
            const uint16_t len=get_be16(block,pos);pos+=2;
            if(!len){terminated=true;break;}
            check(block,pos,len);
            if(stream.size()+len>kMaxXexBytes)bad("LZX: fluxo comprimido excede limite");
            stream.insert(stream.end(),block.begin()+static_cast<std::ptrdiff_t>(pos),
                          block.begin()+static_cast<std::ptrdiff_t>(pos+len));
            pos+=len;
        }
        if(!terminated)bad("LZX: chunk finalizador ausente");
        // Any padding at the end of the outer block must be zero.
        for(;pos<block.size();++pos)if(block[pos]!=0)bad("LZX: lixo no padding do bloco");
        cursor+=block_size;
        block_size=next_size;
        initial_hash=next_hash;
    }
    if(cursor!=input.size())bad("LZX: bytes excedentes apos o ultimo bloco");
    if(stream.empty())bad("LZX: fluxo comprimido vazio");
    return stream;
}

std::vector<uint8_t> decompress_xex_lzx(Bytes data, uint32_t first_block_size,
                                        std::array<uint8_t,20> first_block_hash,
                                        uint32_t window_size,uint32_t image_size){
    if(window_size<(1u<<15)||window_size>(1u<<21)||(window_size&(window_size-1)))
        bad("LZX: janela deve ser potencia de dois entre 32 KiB e 2 MiB");
    unsigned bits=0;for(auto v=window_size;v>1;v>>=1)++bits;
    auto deblocked=unpack_xex_lzx_blocks(data,first_block_size,first_block_hash);
    return decode_lzx(deblocked,image_size,int(bits));
}
} // namespace x360::xex