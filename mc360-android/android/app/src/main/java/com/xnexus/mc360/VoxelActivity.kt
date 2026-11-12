package com.xnexus.mc360

import android.app.Activity
import android.opengl.GLSurfaceView
import android.os.Bundle
import android.view.KeyEvent
import android.view.MotionEvent
import android.widget.FrameLayout
import android.widget.TextView
import javax.microedition.khronos.egl.EGLConfig
import javax.microedition.khronos.opengles.GL10

class VoxelActivity : Activity() {
    private lateinit var surface: DemoSurface
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        surface = DemoSurface()
        val root = FrameLayout(this)
        root.addView(surface)
        root.addView(TextView(this).apply {
            text = "DEMO ORIGINAL — não é Minecraft\nGire com toque ou setas/ombros do controle"
            textSize = 14f
            setPadding(20, 15, 10, 12)
            setBackgroundColor(0x77000000)
        })
        setContentView(root)
    }
    override fun onResume() { super.onResume(); surface.onResume() }
    override fun onPause() { surface.onPause(); super.onPause() }
    override fun onKeyDown(keyCode: Int, event: KeyEvent): Boolean {
        val delta = GamepadControls.yawDelta(keyCode)
        if (delta != 0f) { surface.queueEvent { NativeBridge.rotate(delta) }; return true }
        return super.onKeyDown(keyCode, event)
    }

    private inner class DemoSurface : GLSurfaceView(this@VoxelActivity) {
        private var lastX = 0f
        init {
            setEGLContextClientVersion(2)
            setEGLConfigChooser(8, 8, 8, 8, 16, 0)
            setRenderer(object : Renderer {
                override fun onSurfaceCreated(gl: GL10?, config: EGLConfig?) { NativeBridge.initGraphics() }
                override fun onSurfaceChanged(gl: GL10?, width: Int, height: Int) { NativeBridge.resize(width, height) }
                override fun onDrawFrame(gl: GL10?) { NativeBridge.draw() }
            })
            renderMode = RENDERMODE_CONTINUOUSLY
        }
        override fun onTouchEvent(event: MotionEvent): Boolean {
            if (event.actionMasked == MotionEvent.ACTION_DOWN) { lastX = event.x; return true }
            if (event.actionMasked == MotionEvent.ACTION_MOVE) {
                val dx = event.x - lastX; lastX = event.x
                queueEvent { NativeBridge.rotate(dx * 0.008f) }
            }
            return true
        }
    }
}
