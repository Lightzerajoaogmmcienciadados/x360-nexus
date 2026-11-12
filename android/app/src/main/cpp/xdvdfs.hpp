#pragma once

#include <cstddef>
#include <cstdint>
#include <memory>
#include <string>
#include <unordered_map>
#include <vector>

namespace x360::xdvdfs {

constexpr uint64_t kRamSize = 512ull * 1024 * 1024;
constexpr uint32_t kGuestRamBase = 0x82000000u;
constexpr size_t kPageSize = 4096;
constexpr uint32_t kMaxLoadedXex = 128u * 1024 * 1024;

struct Entry {
    std::string path;
    uint32_t sector = 0;
    uint32_t size = 0;
    bool directory = false;
};

struct Volume {
    std::string layout;
    uint64_t partition_base = 0;
    uint32_t root_sector = 0;
    uint32_t root_size = 0;
    uint64_t image_size = 0;
};

// Reads selected Android Storage Access Framework documents via pread().
// This does not take ownership of fd; caller must keep it open during use.
class FdReader {
public:
    explicit FdReader(int fd);
    uint64_t size() const { return size_; }
    void read(uint64_t offset, void* output, size_t count) const;
private:
    int fd_;
    uint64_t size_;
};

// Read-only Xbox/Xbox 360 disc filesystem. No game decryption or execution.
class Image {
public:
    explicit Image(const FdReader& reader);
    const Volume& volume() const { return volume_; }
    const std::vector<Entry>& entries() const { return entries_; }
    const Entry* find_default_xex() const;
    void read_file(const Entry& file, uint64_t offset, void* dst, size_t size) const;
private:
    const FdReader& reader_;
    Volume volume_;
    std::vector<Entry> entries_;
    uint64_t metadata_bytes_ = 0;
    uint64_t sector_offset(uint32_t sector, uint64_t size) const;
    void parse_directory(uint32_t sector, uint32_t size, const std::string& prefix,
                         unsigned depth, std::vector<uint64_t>& ancestor_dirs);
};

// Sparse backing storage: 512 MiB guest address space, only touched 4 KiB
// pages consume host RAM. This is staging memory, not an Xbox 360 MMU.
class GuestRam {
public:
    void write(uint32_t guest_address, const void* src, size_t size);
    void read(uint32_t guest_address, void* dst, size_t size) const;
    size_t allocated_bytes() const { return pages_.size() * kPageSize; }
private:
    struct Page { uint8_t data[kPageSize]{}; };
    std::unordered_map<uint32_t, std::unique_ptr<Page>> pages_;
    static uint64_t checked_offset(uint32_t guest_address, size_t size);
};

struct LoadResult {
    uint32_t guest_address = kGuestRamBase;
    uint32_t loaded_bytes = 0;
    uint32_t crc32 = 0;
    bool xex2_signature = false;
};

LoadResult load_xex(const Image& image, const Entry& file, GuestRam& ram);
uint64_t copy_file_to_fd(const Image& image, const Entry& file, int output_fd);

} // namespace x360::xdvdfs
