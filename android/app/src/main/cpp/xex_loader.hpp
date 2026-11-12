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
struct ImportLibrary { std::string name; uint32_t record_count=0; std::vector<uint32_t> guest_slots; };
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
                                std::optional<std::array<uint8_t,16>> session_key = std::nullopt);

// PE32 section loader (POWERPCBE machine). The guest RAM base may overlap the
// raw XEX staging region; pass the extracted PE in a separate host vector.
PreparedImage map_pe(std::span<const uint8_t> pe, const Headers& headers,
                     x360::xdvdfs::GuestRam& ram);

// Reusable AES-128 CBC primitive; caller supplies a lawful 16-byte test key.
// Ciphertext must be a whole number of AES blocks. No implicit PKCS#7 removal.
std::vector<uint8_t> aes128_cbc_decrypt(std::span<const uint8_t> ciphertext,
                                        const std::array<uint8_t,16>& key);
} // namespace x360::xex