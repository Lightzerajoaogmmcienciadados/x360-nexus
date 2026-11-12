// Nexus Blocks: original CPU voxel raycaster for ARM64 Android.
// No Minecraft code/assets, no Xbox 360/Xenos shader translation.
#include <jni.h>
#include <cmath>
#include <algorithm>
#include <cstdint>
#include <limits>
#include <vector>
namespace {
constexpr int WX=24, WY=12, WZ=24;
constexpr float FAR_PLANE=24.0f;
struct V3 {float x,y,z;};
static inline V3 add(V3 a,V3 b){return {a.x+b.x,a.y+b.y,a.z+b.z};}
static inline V3 mul(V3 a,float s){return {a.x*s,a.y*s,a.z*s};}
static inline V3 normalize(V3 d){
  float len=std::sqrt(d.x*d.x+d.y*d.y+d.z*d.z);
  if(len<0.00001f)return {0.f,0.f,1.f};
  return {d.x/len,d.y/len,d.z/len};
}
static inline jint rgb(float r,float g,float b){
  auto b8=[](float c)->uint32_t {
    return (uint32_t)std::clamp(c,0.f,255.f);
  };
  return (jint)(0xff000000u|(b8(r)<<16)|(b8(g)<<8)|b8(b));
}
static inline int cell(int x,int y,int z){
  return x+WX*(z+WZ*y);
}
static inline bool inside(int x,int y,int z){
  return x>=0&&x<WX&&y>=0&&y<WY&&z>=0&&z<WZ;
}
static inline float inv(float v){
  return std::fabs(v)<0.000001f?1e10f:1.f/std::fabs(v);
}
static inline float entry(float o,float d,int i){
  if(std::fabs(d)<0.000001f)return 1e10f;
  return d>0.f?(i+1.f-o)/d:(o-i)/(-d);
}
static inline V3 block_color(uint8_t type,int face,float hitX,float hitY,float hitZ){
  V3 c;
  switch(type){
    case 1: c=face==1?V3{71,185,77}:V3{112,140,69};break;  // grass
    case 2: c={118,82,56};break;                            // soil
    case 3: c={115,124,130};break;                          // rock
    case 4: c={149,106,64};break;                           // wood
    case 5: c={47,128,67};break;                            // leaves
    case 6: c={213,189,116};break;                          // sand
    default:c={220,220,220};break;
  }
  float feature=std::sin(hitX*15.7f+hitY*9.2f+hitZ*11.4f)*5.2f;
  c.x+=feature;c.y+=feature;c.z+=feature;
  return c;
}
static jint render_ray(const uint8_t*blocks,V3 origin,V3 direction,float skyAlpha){
  const int skyR=(int)(51+80*skyAlpha),skyG=(int)(126+60*skyAlpha),
            skyB=(int)(160+70*skyAlpha);
  const jint sky=rgb((float)skyR,(float)skyG,(float)skyB);
  int x=(int)std::floor(origin.x),y=(int)std::floor(origin.y),
      z=(int)std::floor(origin.z);
  int stepX=direction.x>=0?1:-1,stepY=direction.y>=0?1:-1,
      stepZ=direction.z>=0?1:-1;
  const float dx=inv(direction.x),dy=inv(direction.y),dz=inv(direction.z);
  float tx=entry(origin.x,direction.x,x);
  float ty=entry(origin.y,direction.y,y);
  float tz=entry(origin.z,direction.z,z);
  int face=1;
  float travel=0.0f;
  for(int iter=0;iter<96&&travel<FAR_PLANE;++iter){
    if(!inside(x,y,z))break;
    uint8_t kind=blocks[cell(x,y,z)];
    if(kind!=0){
      V3 point=add(origin,mul(direction,travel));
      V3 color=block_color(kind,face,point.x,point.y,point.z);
      float light=face==1?0.99f:face==0?0.77f:0.87f;
      const float fog=std::clamp(travel/32.0f,0.0f,0.68f);
      const float lit=(1.f-fog)*light;
      return rgb(color.x*lit+skyR*fog,
                 color.y*lit+skyG*fog,
                 color.z*lit+skyB*fog);
    }
    if(tx<=ty&&tx<=tz){x+=stepX;travel=tx;tx+=dx;face=0;}
    else if(ty<=tz){y+=stepY;travel=ty;ty+=dy;face=1;}
    else{z+=stepZ;travel=tz;tz+=dz;face=2;}
  }
  return sky;
}
}  // namespace

extern "C" JNIEXPORT jintArray JNICALL
Java_com_x360nexus_VoxelRenderer_nativeRender(
  JNIEnv* env,jclass, jbyteArray world,jfloat px,jfloat py,jfloat pz,
  jfloat yaw,jfloat pitch,jint width,jint height){
  if(!world||env->GetArrayLength(world)!=WX*WY*WZ||
      width<100||height<60||width>320||height>180||
      !std::isfinite(px)||!std::isfinite(py)||!std::isfinite(pz)||
      !std::isfinite(yaw)||!std::isfinite(pitch))return nullptr;
  std::vector<jbyte> raw(WX*WY*WZ);
  env->GetByteArrayRegion(world,0,(jsize)raw.size(),raw.data());
  if(env->ExceptionCheck())return nullptr;
  const auto *blocks=reinterpret_cast<const uint8_t*>(raw.data());
  V3 origin={px,py,pz};
  const float s=std::sin(yaw),c=std::cos(yaw),
              sp=std::sin(pitch),cp=std::cos(pitch);
  V3 forward={s*cp,sp,c*cp};
  V3 right={c,0.f,-s};
  V3 up={-s*sp,cp,-c*sp};
  std::vector<jint> pixels((size_t)width*(size_t)height);
  const float scale=0.95f;
  const float aspect=(float)height/width;
  for(int row=0;row<height;++row){
    const float ny=(1.f-2.f*(row+0.5f)/height)*scale*aspect;
    for(int col=0;col<width;++col){
      const float nx=(2.f*(col+0.5f)/width-1.f)*scale;
      V3 dir=normalize(add(add(forward,mul(right,nx)),mul(up,ny)));
      pixels[(size_t)row*width+col]=render_ray(
        blocks,origin,dir,std::clamp(1.f-(float)row/height,0.f,1.f));
    }
  }
  // Center reticle, with dark outline for both light and dark backgrounds.
  const int cx=width/2,cy=height/2;
  for(int i=-6;i<=6;++i){
    if(cx+i>=0&&cx+i<width){
      pixels[(size_t)cy*width+cx+i]=(i==0)?0xff10271d:0xffedfff5;
    }
    if(cy+i>=0&&cy+i<height){
      pixels[(size_t)(cy+i)*width+cx]=(i==0)?0xff10271d:0xffedfff5;
    }
  }
  jintArray output=env->NewIntArray((jsize)pixels.size());
  if(!output)return nullptr;
  env->SetIntArrayRegion(output,0,(jsize)pixels.size(),pixels.data());
  return output;
}
