//! JNI access to the bounded Xenos-inspired GPU diagnostic rasterizer.
//! Return a copy of a 320x180 ARGB preview: never expose raw native pointers.
use jni::{JNIEnv,objects::JObject,sys::{jint,jintArray,jstring}};
use crate::{xenos_lab,xenos_pipeline};
#[no_mangle]
pub extern "system" fn Java_com_x360nexus_MainActivity_nativeGpuLabFrame(
    mut env:JNIEnv,_:JObject,mode:jint
)->jintArray {
    let values:Vec<i32>=if mode<3 {
        let Ok(frame)=xenos_lab::demo(mode) else{return std::ptr::null_mut()};
        frame.pixels().iter().map(|&c|c as i32).collect()
    } else {
        let Ok(frame)=xenos_pipeline::demo(mode-3) else{return std::ptr::null_mut()};
        frame.pixels().iter().map(|&c|c as i32).collect()
    };
    let Ok(array)=env.new_int_array(values.len() as i32) else {return std::ptr::null_mut()};
    if env.set_int_array_region(&array,0,&values).is_err(){return std::ptr::null_mut()}
    array.into_raw()
}
#[no_mangle]
pub extern "system" fn Java_com_x360nexus_MainActivity_nativeGpuLabReport(
    mut env:JNIEnv,_:JObject,mode:jint
)->jstring {
    let report=if mode<3 {
        xenos_lab::demo(mode).map(|g|g.summary())
    } else {
        xenos_pipeline::demo(mode-3).map(|g|g.summary())
    }.unwrap_or_else(|err|format!("GPU LAB erro: {err}"));
    env.new_string(report).map(|s|s.into_raw()).unwrap_or(std::ptr::null_mut())
}
