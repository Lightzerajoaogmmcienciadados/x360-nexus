
use jni::JNIEnv;
use jni::objects::{JClass, JString};
use jni::sys::jstring;

#[no_mangle]
pub extern "system" fn
Java_com_x360nexus_MainActivity_nativeVersion(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    let result: JString = env
        .new_string("X360 Nexus Rust Core 0.2 - ativo")
        .expect("Falha ao criar string");

    result.into_raw()
}
