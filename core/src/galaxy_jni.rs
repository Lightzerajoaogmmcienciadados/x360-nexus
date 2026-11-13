//! JNI bridge for ISO inspection, XEX metadata, demo execution and controller state.
use jni::{JNIEnv,objects::JObject,sys::{jint,jboolean,jstring}};
use std::{fs::File,os::fd::FromRawFd,sync::{Mutex,OnceLock}};
use crate::{loader,input::Controller};
static PAD:OnceLock<Mutex<Controller>>=OnceLock::new();
fn pad()->&'static Mutex<Controller>{PAD.get_or_init(||Mutex::new(Controller::default()))}
/// Snapshot shared by touchscreen/USB/Bluetooth input and XamInputGetState HLE.
/// If a controller lock is poisoned, return a released neutral state.
pub(crate) fn controller_snapshot()->Controller{
    pad().lock().map(|state|*state).unwrap_or_default()
}
fn output(mut env:JNIEnv,msg:String)->jstring{env.new_string(msg).map(|s|s.into_raw()).unwrap_or(std::ptr::null_mut())}
#[no_mangle]pub extern "system" fn Java_com_x360nexus_MainActivity_nativeInspectIso(env:JNIEnv,_:JObject,fd:jint)->jstring{
 if fd<0{return output(env,"FD invalido".into())}
 // Java transfers ownership of a duplicated descriptor.
 let mut f=unsafe{File::from_raw_fd(fd)};
 let msg=match loader::inspect_iso(&mut f){Ok(i)=>{
  let names=match loader::root_entries(&mut f,&i){Ok(entries)=>entries.iter().take(40).map(|e|format!("{} {} ({} bytes)",if e.is_directory{"[DIR]"}else{"[FILE]"},e.name,e.size)).collect::<Vec<_>>().join("\n"),Err(e)=>format!("Diretorio nao lido: {e}")};
  format!("XDVDFS detectado: {}\nImagem: {} bytes\nParticao: 0x{:X}\nRaiz: setor {}\n\n{}\n\nExecucao comercial ainda indisponivel",i.layout,i.image_size,i.partition,i.root_sector,names)
 },Err(e)=>format!("ISO nao reconhecida: {e}")};output(env,msg)
}
#[no_mangle]pub extern "system" fn Java_com_x360nexus_MainActivity_nativeRunDemo(env:JNIEnv,_:JObject)->jstring{output(env,crate::galaxy::run_demo().unwrap_or_else(|e|format!("Erro: {e}")))}
#[no_mangle]pub extern "system" fn Java_com_x360nexus_MainActivity_nativeSetButton(_env:JNIEnv,_:JObject,mask:jint,pressed:jboolean){if let Ok(mut c)=pad().lock(){if pressed!=0{c.buttons|=mask as u32}else{c.buttons&=!(mask as u32)}}}
#[no_mangle]pub extern "system" fn Java_com_x360nexus_MainActivity_nativeSetAxis(_env:JNIEnv,_:JObject,axis:jint,value:jint){if let Ok(mut c)=pad().lock(){let v=value.clamp(-32768,32767) as i16;match axis{0=>c.left_x=v,1=>c.left_y=v,2=>c.right_x=v,3=>c.right_y=v,4=>c.left_trigger=value.clamp(0,255) as u8,5=>c.right_trigger=value.clamp(0,255) as u8,_=>()}}}
#[no_mangle]pub extern "system" fn Java_com_x360nexus_MainActivity_nativePadStatus(env:JNIEnv,_:JObject)->jstring{let s=pad().lock().map(|c|format!("Botoes: {:014b}\nL: {},{} R: {},{}\nLT={} RT={}",c.buttons,c.left_x,c.left_y,c.right_x,c.right_y,c.left_trigger,c.right_trigger)).unwrap_or_else(|_|"Erro no controle".into());output(env,s)}
