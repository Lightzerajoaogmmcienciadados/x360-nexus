#pragma once
#include "xdvdfs.hpp"
#include <array>
#include <cstdint>
#include <optional>
#include <span>
#include <string>
#include <vector>

namespace x360::xex {
// XEX2 header fields are big-endian; the contained PE is little-endian.
// Bounds and size caps intentionally prevent malformed input from allocating
// unbounded amounts of RAM on a phone.
inline constexpr size_t kMaxXexBytes = 128ull * 1024 * 1024;
inline constexpr size_t kMaxPeBytes = 128ull * 1024 * 1024;
struct OptionalHeader { uint32_t key=0, value=0; };
struct ImportLibrary { std::string name; uint16_t name_index=0; uint32_t record_count=0; std::vector<uint32_t> guest_slots; };
struct Headers {
    uint32_t module_flags=0, pe_offset=0, security_offset=0;
    uint32_t image_base=0, entry_point=0, image_size=0;
    uint32_t compression=0, encryption=0, page_count=0;
    uint32_t title_id=0;
    std::vector<OptionalHeader> options;
    std::vector<ImportLibrary> imports;
    uint32_t file_format_offset=0;
    uint32_t file_format_size=0;
};
// XEX import records are *addresses in the loaded PE*, not ordinal IDs.
// A type-0 record is a variable entry; type-1 is an executable thunk.
struct ImportThunk {
    uint32_t address=0;
    uint32_t ordinal=0;
    uint32_t module=0; // 1=xboxkrnl.exe, 2=xam.xex
    bool inferred_from_veneer=false; // validated PPC import veneer, linked by ordinal
    uint32_t linked_variable_slot=0; // verified type-0 import slot used by an indirect veneer
};
struct ImportVariable {
    uint32_t address=0;
    uint32_t ordinal=0;
    uint32_t module=0; // 1=xboxkrnl.exe, 2=xam.xex
};
struct ImportInventory {
    std::vector<ImportThunk> thunks;
    std::vector<ImportVariable> variables_found;
    size_t records=0, variables=0, malformed=0, unsupported_libraries=0;
    size_t unrecognized_types=0, uninitialized=0, executable_candidates=0, paired_veneers=0;
    size_t scanned_veneers=0, scanned_executable_bytes=0;
    std::vector<std::string> examples; // first eight unknown descriptors, never keys

    std::string describe() const;
};
ImportInventory inspect_import_thunks(const std::vector<ImportLibrary>& libs,
    uint32_t image_base, uint32_t image_size, const x360::xdvdfs::GuestRam& ram,
    const std::vector<std::pair<uint32_t,uint32_t>>& executable_ranges = {});
// Patch only proven type-1 XEX2 executable thunk slots, never unknown records.
// The Rust CPU uses the separate module/ordinal catalog to dispatch HLE.
size_t patch_import_thunks(const ImportInventory& inv,
    const std::vector<std::pair<uint32_t,uint32_t>>& executable_ranges,
    x360::xdvdfs::GuestRam& ram);

struct PeSection {
    std::string name;
    uint32_t virtual_address=0, virtual_size=0;
    uint32_t raw_address=0, raw_size=0, characteristics=0;
};
struct PreparedImage {
    uint32_t base=0, pc=0, image_size=0;
    std::vector<PeSection> sections;
    std::vector<ImportLibrary> imports;
    // Only these ranges may be fetched as executable code in the tracer.
    std::vector<std::pair<uint32_t,uint32_t>> executable_ranges;
    bool can_fetch(uint32_t address, size_t byte_count) const;
};

Headers parse_headers(std::span<const uint8_t> xex);
std::string describe(const Headers& info);

// Optional *session* key for developer-owned test executables; NO retail or
// device keys are shipped with this project. The encrypted key in XEX2's
// security block is not itself the unwrapped AES session key.
std::vector<uint8_t> extract_pe(std::span<const uint8_t> xex,
                                const Headers& headers,
                                std::optional<std::array<uint8_t,16>> master_key = std::nullopt);

// The 16-byte key is supplied at runtime by the user. The encrypted image
// key resides at security_offset+0x150; no console key is distributed.
std::array<uint8_t,16> derive_session_key(std::span<const uint8_t> xex,
                                           const Headers& headers,
                                           const std::array<uint8_t,16>& master_key);

// LZX Normal: verify the linked block chain (SHA-1), remove XEX chunk framing,
// then decode the original CAB-style LZX Huffman/offset stream.
std::array<uint8_t,20> sha1_lzx_block(std::span<const uint8_t> data);
std::vector<uint8_t> unpack_xex_lzx_blocks(std::span<const uint8_t> input,
    uint32_t first_block_size, std::array<uint8_t,20> first_block_hash);
std::vector<uint8_t> decompress_xex_lzx(std::span<const uint8_t> data,
    uint32_t first_block_size, std::array<uint8_t,20> first_block_hash,
    uint32_t window_size,uint32_t image_size);

// PE32 section loader (POWERPCBE machine). The guest RAM base may overlap the
// raw XEX staging region; pass the extracted PE in a separate host vector.
PreparedImage map_pe(std::span<const uint8_t> pe, const Headers& headers,
                     x360::xdvdfs::GuestRam& ram);

// Reusable AES-128 CBC primitive; caller supplies a lawful 16-byte test key.
// Ciphertext must be a whole number of AES blocks. No implicit PKCS#7 removal.
std::vector<uint8_t> aes128_cbc_decrypt(std::span<const uint8_t> ciphertext,
                                        const std::array<uint8_t,16>& key);
} // namespace x360::xex