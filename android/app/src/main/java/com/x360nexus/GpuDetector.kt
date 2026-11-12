package com.x360nexus

import android.content.Context
import android.opengl.EGL14
import android.opengl.GLES20

/** Queries a temporary GLES context and Android Vulkan feature; doesn't render. */
object GpuDetector {
    data class Info(val renderer: String, val version: String, val vulkan: String, val family: String)
    fun query(context: Context): Info {
        var renderer = "Indisponível"
        var gles = "Não detectado"
        try {
            val display = EGL14.eglGetDisplay(EGL14.EGL_DEFAULT_DISPLAY)
            if (display != EGL14.EGL_NO_DISPLAY) {
                val versions = IntArray(2)
                if (EGL14.eglInitialize(display, versions, 0, versions, 1)) {
                    val attrs = intArrayOf(EGL14.EGL_RENDERABLE_TYPE, EGL14.EGL_OPENGL_ES2_BIT,
                        EGL14.EGL_SURFACE_TYPE, EGL14.EGL_PBUFFER_BIT, EGL14.EGL_NONE)
                    val configs = arrayOfNulls<android.opengl.EGLConfig>(1)
                    val count = IntArray(1)
                    if (EGL14.eglChooseConfig(display, attrs, 0, configs, 0, 1, count, 0) && count[0] > 0) {
                        val config = configs[0]!!
                        val pbuffer = EGL14.eglCreatePbufferSurface(display, config,
                            intArrayOf(EGL14.EGL_WIDTH, 1, EGL14.EGL_HEIGHT, 1, EGL14.EGL_NONE), 0)
                        var context3 = EGL14.eglCreateContext(display, config, EGL14.EGL_NO_CONTEXT,
                            intArrayOf(EGL14.EGL_CONTEXT_CLIENT_VERSION, 3, EGL14.EGL_NONE), 0)
                        if (context3 == EGL14.EGL_NO_CONTEXT) {
                            context3 = EGL14.eglCreateContext(display, config, EGL14.EGL_NO_CONTEXT,
                                intArrayOf(EGL14.EGL_CONTEXT_CLIENT_VERSION, 2, EGL14.EGL_NONE), 0)
                        }
                        if (pbuffer != EGL14.EGL_NO_SURFACE && context3 != EGL14.EGL_NO_CONTEXT) {
                            if (EGL14.eglMakeCurrent(display, pbuffer, pbuffer, context3)) {
                                renderer = GLES20.glGetString(GLES20.GL_RENDERER) ?: renderer
                                gles = GLES20.glGetString(GLES20.GL_VERSION) ?: gles
                            }
                            EGL14.eglMakeCurrent(display, EGL14.EGL_NO_SURFACE, EGL14.EGL_NO_SURFACE, EGL14.EGL_NO_CONTEXT)
                        }
                        if (context3 != EGL14.EGL_NO_CONTEXT) EGL14.eglDestroyContext(display, context3)
                        if (pbuffer != EGL14.EGL_NO_SURFACE) EGL14.eglDestroySurface(display, pbuffer)
                    }
                    EGL14.eglTerminate(display)
                }
            }
        } catch (_: Throwable) { /* A driver may not expose EGL on this device. */ }
        val vkPacked = context.packageManager.systemAvailableFeatures
            .firstOrNull { it.name == "android.hardware.vulkan.version" }?.version ?: 0
        val major = vkPacked ushr 22
        val minor = (vkPacked ushr 12) and 0x3FF
        val vk = if (vkPacked == 0) "Não anunciado" else "$major.$minor (Android feature)"
        val family = when {
            renderer.contains("Adreno", true) -> "Qualcomm Adreno"
            renderer.contains("Mali", true) -> "ARM Mali"
            renderer.contains("Xclipse", true) -> "Samsung Xclipse"
            renderer.contains("PowerVR", true) -> "PowerVR"
            else -> "GPU não classificada"
        }
        return Info(renderer, gles, vk, family)
    }
}