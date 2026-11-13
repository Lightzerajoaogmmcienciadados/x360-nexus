#pragma once
#include <array>
#include <cstddef>
#include <cstdint>
#include <deque>
#include <functional>
#include <vector>
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
    uint64_t event_writes = 0;
    uint64_t alu_loads = 0;
    uint64_t nop_packets = 0;
    std::string last_event;
};
// Bounded PM4 packet parser for diagnostics. This is NOT the Xenos rasterizer
// or a DMA/MMIO implementation. Words are already host-decoded BE dwords.
class CommandProcessor {
public:
    static constexpr size_t kMaxQueuedWords = 16384;
    // Read words from memory owned by the guest, never from an unchecked host pointer.
    using GuestWords = std::function<bool(uint32_t, size_t, std::vector<uint32_t>&)>;
    bool submit(std::span<const uint32_t> words, std::string& error,
                const GuestWords& read_guest = {});
    Stats stats() const;
    bool register_value(uint32_t index, uint32_t& value) const;
    bool diagnostic_clear_color(uint32_t& rgba) const;
private:
    mutable std::mutex mutex_;
    std::deque<uint32_t> queue_;
    std::array<uint32_t, 0x10000> regs_{};
    Stats stats_;
    bool diagnostic_color_set_ = false;
    void consume_locked(const GuestWords& read_guest);
    void write_register(uint32_t index, uint32_t value);
};
CommandProcessor& processor();
}  // namespace x360::gpu