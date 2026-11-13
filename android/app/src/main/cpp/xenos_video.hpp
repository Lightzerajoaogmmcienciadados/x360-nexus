#pragma once
#include <string>
namespace x360::gpu {
void set_video_options(int backend,int scale,bool vsync);
std::string video_status();
}
