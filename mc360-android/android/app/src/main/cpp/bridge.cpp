#include <jni.h>
#include <GLES2/gl2.h>
#include <algorithm>
#include <cstdint>
#include <string>
#include <vector>
#include <cctype>
#include <cstdio>
#include <cmath>
extern "C" uint32_t mc360_probe(const uint8_t*, size_t);
extern "C" int32_t mc360_height(int32_t, int32_t);
namespace {
GLuint program = 0, vbo = 0;
GLint aPos = -1, aCol = -1, uYaw = -1, uAspect = -1;
GLsizei count = 0;
float yaw = 0.6f, aspect = 1.f;
std::vector<float> verts;
const char* vertex = R"(attribute vec3 p; attribute vec3 c;
 uniform float yaw; uniform float aspect; varying vec3 color;
 void main() {
   float ca = cos(yaw), sa = sin(yaw);
   vec3 q = p - vec3(6.0,1.5,6.0);
   vec3 r = vec3(q.x*ca-q.z*sa, q.y, q.x*sa+q.z*ca);
   gl_Position = vec4(r.x*0.11/max(1.0,aspect), r.y*0.11-r.z*0.065, 0.32+r.z*0.025, 1.0);
   color=c;
 })";
const char* frag = R"(precision mediump float; varying vec3 color;
 void main(){ gl_FragColor = vec4(color,1.0); })";
void vertex3(float x,float y,float z,float r,float g,float b) {
  verts.insert(verts.end(),{x,y,z,r,g,b});
}
void quad(float x,float y,float z,float X,float Y,float Z,int face,float r,float g,float b) {
  float p[4][3];
  if (face==0) { float q[4][3]={{x,Y,z},{X,Y,z},{X,Y,Z},{x,Y,Z}};std::copy(&q[0][0],&q[0][0]+12,&p[0][0]); }
  else if(face==1) { float q[4][3]={{X,y,z},{X,Y,z},{X,Y,Z},{X,y,Z}};std::copy(&q[0][0],&q[0][0]+12,&p[0][0]); }
  else { float q[4][3]={{x,y,Z},{X,y,Z},{X,Y,Z},{x,Y,Z}};std::copy(&q[0][0],&q[0][0]+12,&p[0][0]); }
  for(int k: {0,1,2,0,2,3}) vertex3(p[k][0],p[k][1],p[k][2],r,g,b);
}
GLuint shader(GLenum type,const char* src) {
  GLuint s=glCreateShader(type); glShaderSource(s,1,&src,nullptr); glCompileShader(s);
  GLint ok=GL_FALSE; glGetShaderiv(s,GL_COMPILE_STATUS,&ok);
  if(!ok){glDeleteShader(s);return 0;} return s;
}
}
extern "C" JNIEXPORT jstring JNICALL
Java_com_xnexus_mc360_NativeBridge_inspect(JNIEnv* e,jobject,jbyteArray bytes,jstring name,jlong size) {
  if(!bytes||!name) return e->NewStringUTF("Dados invalidos");
  jsize length=e->GetArrayLength(bytes);
  jbyte* ptr=e->GetByteArrayElements(bytes,nullptr);
  const char* nm=e->GetStringUTFChars(name,nullptr);
  if(!ptr||!nm){
    if(ptr)e->ReleaseByteArrayElements(bytes,ptr,JNI_ABORT);
    if(nm)e->ReleaseStringUTFChars(name,nm);
    return e->NewStringUTF("Falha de leitura");
  }
  uint32_t id=mc360_probe(reinterpret_cast<uint8_t*>(ptr),static_cast<size_t>(length));
  std::string filename(nm);
  std::transform(filename.begin(),filename.end(),filename.begin(),[](unsigned char c){return (char)std::tolower(c);});
  std::string result=id==1?"Cabecalho XEX2 encontrado":id==2?"Pacote STFS reconhecido":id==3?"Arquivo ZIP reconhecido":"Tipo desconhecido";
  if(id==0 && filename.size()>4 && filename.substr(filename.size()-4)==".iso") result="Extensao ISO: conteudo nao verificado";
  if(size>=0){char buffer[100];std::snprintf(buffer,sizeof(buffer)," (%.2f GiB)",(double)size/1073741824.0);result+=buffer;}
  result+=". Somente os primeiros 4 KiB foram analisados. Minecraft NAO foi executado.";
  e->ReleaseByteArrayElements(bytes,ptr,JNI_ABORT); e->ReleaseStringUTFChars(name,nm);
  return e->NewStringUTF(result.c_str());
}
extern "C" JNIEXPORT void JNICALL
Java_com_xnexus_mc360_NativeBridge_initGraphics(JNIEnv*,jobject) {
  GLuint vs=shader(GL_VERTEX_SHADER,vertex),fs=shader(GL_FRAGMENT_SHADER,frag);
  if(!vs||!fs) return;
  program=glCreateProgram();glAttachShader(program,vs);glAttachShader(program,fs);glLinkProgram(program);
  glDeleteShader(vs);glDeleteShader(fs);
  GLint ok=GL_FALSE;glGetProgramiv(program,GL_LINK_STATUS,&ok);
  if(!ok){glDeleteProgram(program);program=0;return;}
  aPos=glGetAttribLocation(program,"p");aCol=glGetAttribLocation(program,"c");
  uYaw=glGetUniformLocation(program,"yaw");uAspect=glGetUniformLocation(program,"aspect");
  verts.clear();
  for(int x=0;x<12;++x) for(int z=0;z<12;++z) {
    float h=(float)mc360_height(x,z)*0.35f;
    quad((float)x,0,(float)z,(float)x+1,h,(float)z+1,0,0.22f,0.62f,0.3f);
    quad((float)x,0,(float)z,(float)x+1,h,(float)z+1,1,0.42f,0.31f,0.18f);
    quad((float)x,0,(float)z,(float)x+1,h,(float)z+1,2,0.50f,0.38f,0.21f);
  }
  count=(GLsizei)(verts.size()/6);
  glGenBuffers(1,&vbo);glBindBuffer(GL_ARRAY_BUFFER,vbo);
  glBufferData(GL_ARRAY_BUFFER,(GLsizeiptr)(verts.size()*sizeof(float)),verts.data(),GL_STATIC_DRAW);
  glEnable(GL_DEPTH_TEST);glClearColor(0.42f,0.74f,0.94f,1.0f);
}
extern "C" JNIEXPORT void JNICALL
Java_com_xnexus_mc360_NativeBridge_resize(JNIEnv*,jobject,jint w,jint h) {
  aspect=(float)std::max(1,w)/(float)std::max(1,h);glViewport(0,0,w,h);
}
extern "C" JNIEXPORT void JNICALL
Java_com_xnexus_mc360_NativeBridge_rotate(JNIEnv*,jobject,jfloat delta) {yaw+=delta;}
extern "C" JNIEXPORT void JNICALL
Java_com_xnexus_mc360_NativeBridge_draw(JNIEnv*,jobject) {
  glClear(GL_COLOR_BUFFER_BIT|GL_DEPTH_BUFFER_BIT);
  if(!program||!vbo||count<=0)return;
  glUseProgram(program);glUniform1f(uYaw,yaw);glUniform1f(uAspect,aspect);
  glBindBuffer(GL_ARRAY_BUFFER,vbo);
  glEnableVertexAttribArray((GLuint)aPos);glEnableVertexAttribArray((GLuint)aCol);
  glVertexAttribPointer((GLuint)aPos,3,GL_FLOAT,GL_FALSE,6*sizeof(float),(void*)0);
  glVertexAttribPointer((GLuint)aCol,3,GL_FLOAT,GL_FALSE,6*sizeof(float),(void*)(3*sizeof(float)));
  glDrawArrays(GL_TRIANGLES,0,count);
  glDisableVertexAttribArray((GLuint)aPos);glDisableVertexAttribArray((GLuint)aCol);
}
