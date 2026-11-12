#pragma once
#include <string>
namespace x360::gpu {
// Settings affect the diagnostic GLES output only; Vulkan/Xenos unavailable.
void set_video_options(int backend, int scale, bool vsync);
std::string video_status();
}