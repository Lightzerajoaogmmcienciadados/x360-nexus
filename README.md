# X360: GALAXY v5.4 — Xenon startup diagnostics and guest memory

Version 5.4 improves the verified v5.3 baseline without bundling games or keys.

## Confirmed structural changes

- A dedicated **sparse low virtual-memory mapping** supplements the existing 512 MiB high XEX image window. Only explicitly mapped low pages can be accessed; null pointers, guard pages and unmapped addresses remain faults.
- The main PowerPC CPU thread now starts with an aligned `r1` stack pointer (`0x700FFF00`) and an initialized `r13` KPCR/TLS pointer (`0x71000000`). The compact KPCR follows documented offsets: TLS at `+0x00`, PCR self-pointer `+0x30`, stack high/low pointers `+0x70/+0x74`, thread pointer `+0x100`. The main TLS block is at `0x71010000` and the thread object at `0x71001000`.
- Additional cooperative test threads receive their **own stack and PCR** rather than sharing the main stack. Low guest heap allocations begin at `0x20000000` and map on demand.
- On a PowerPC fault, diagnostics include PC, raw big-endian opcode, decoded D/DS/X-form instruction, effective memory address (EA), r1, r3..r6, r10/r11, r13, r30 and r31. This is particularly useful for the reported `lwz r11,0x94(r30)` at `0x8214AB14`.
- The XEX import decoder distinguishes **invalid records, unknown high-byte types, and all-zero/uninitialized import data**. Type-1 thunks are patched to an `sc 2`/NOP/BLR trampoline *only if* their descriptor and executable 16-byte destination are validated. The original import library/ordinal catalogue is cached for the Rust dispatcher so the patch doesn't erase it. No heuristic guesses or fabricated 142 imports.
- The Rust HLE adds verified Xbox ordinals for low guest virtual-memory allocation/release (204/220), simple critical-section operations (293/302/304), TLS (338/340/341), simplified event primitives, and test-only video/ring services. Unsupported services remain visible failures. No unconditional `r3=0` stub blanket.
- The Android root inset listener now accounts for status and navigation bar heights explicitly, with extra safety padding under the system clock. Version markers, Cargo and CI are set to v5.4.
- Existing `VdGetSystemCommandBuffer` test ring at `0x9F000000` and `VdSwap`-gated PM4 submission to the GLES decoder remain connected through JNI and the Kotlin telemetry poller.

## Deliberate limitations

This is not a commercial-game-ready Xenon kernel, Xenos shader compiler, or Vulkan backend. In particular, **r30 is not an ABI-defined initial register**: setting it to Image Base to bypass a fault could hide a real loader/entry mismatch. The diagnostic now prints the first failing instruction and EA. Unknown import descriptor types cannot safely be reclassified as calls without the actual bytes. Xbox Live services, EDRAM, MMIO, VMX128, full CRT and thread ABI remain incomplete. CI only verifies compilation and synthetic unit tests; Minecraft boot requires an on-device test.

No console keys or retail game content are provided. Use independently acquired and lawful test inputs.