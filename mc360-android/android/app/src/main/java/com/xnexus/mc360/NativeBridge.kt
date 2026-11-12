package com.xnexus.mc360

object NativeBridge {
    init { System.loadLibrary("mc360port") }
    external fun inspect(prefix: ByteArray, filename: String, size: Long): String
    external fun initGraphics()
    external fun resize(width: Int, height: Int)
    external fun draw()
    external fun rotate(delta: Float)
}
