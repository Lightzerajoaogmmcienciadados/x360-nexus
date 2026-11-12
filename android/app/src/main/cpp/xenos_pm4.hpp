#pragma once
#include <array>
#include <cstddef>
#include <cstdint>
#include <deque>
#include <mutex>
#include <span>
#include <string>

namespace x360::gpu {
struct Stats {
    uint64_t packets = 0;
    uint64_t register_writes = 0;
    uint64_t draws_seen = 0;
    uint64_t unsupported = 0;
    uint64_t frames = 0;
    std::string last_event;
};
// Bounded PM4 packet parser for diagnostics. This is NOT the Xenos rasterizer
// or a DMA/MMIO implementation. Words are already host-decoded BE dwords.
class CommandProcessor {
public:
    static constexpr size_t kMaxQueuedWords = 16384;
    bool submit(std::span<const uint32_t> words, std::string& error);
    Stats stats() const;
    bool register_value(uint32_t index, uint32_t& value) const;
private:
    mutable std::mutex mutex_;
    std::deque<uint32_t> queue_;
    std::array<uint32_t, 0x10000> regs_{};
    Stats stats_;
    void consume_locked();
    void write_register(uint32_t index, uint32_t value);
};
CommandProcessor& processor();
}  // namespace x360::gpu