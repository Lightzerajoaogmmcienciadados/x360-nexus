package com.x360nexus

import android.content.Context
import java.io.File
import java.io.IOException

/**
 * Nexus Blocks user settings and local saves; ORIGINAL sandbox only.
 * Uses the app's private storage, never reads/modifies Minecraft's content.
 */
object VoxelSaves {
    private const val PREF = "nexus_blocks_prefs"
    private const val FILE_NAME = "nexus_blocks_world.nvox"
    private const val MIN_LEN = 30 + 24 * 12 * 24

    @JvmStatic
    fun renderWidth(context: Context): Int {
        val raw = context.getSharedPreferences(PREF, Context.MODE_PRIVATE)
            .getInt("render_width", 240)
        return when (raw) {
            160, 240, 320 -> raw
            else -> 240
        }
    }

    @JvmStatic
    fun setRenderWidth(context: Context, width: Int) {
        if (width == 160 || width == 240 || width == 320) {
            context.getSharedPreferences(PREF, Context.MODE_PRIVATE)
                .edit().putInt("render_width", width).apply()
        }
    }

    @JvmStatic
    fun frameLimit(context: Context): Int {
        val fps = context.getSharedPreferences(PREF, Context.MODE_PRIVATE)
            .getInt("fps", 30)
        return if (fps == 20 || fps == 30 || fps == 60) fps else 30
    }

    @JvmStatic
    fun setFrameLimit(context: Context, fps: Int) {
        if (fps == 20 || fps == 30 || fps == 60) {
            context.getSharedPreferences(PREF, Context.MODE_PRIVATE)
                .edit().putInt("fps", fps).apply()
        }
    }

    @JvmStatic
    fun save(context: Context, contents: ByteArray?): Boolean {
        if (contents == null || contents.size != MIN_LEN ||
            !contents.copyOfRange(0, 4).contentEquals(byteArrayOf(78, 86, 79, 88))
        ) return false
        val destination = File(context.filesDir, FILE_NAME)
        val temporary = File(context.filesDir, "$FILE_NAME.tmp")
        return try {
            temporary.writeBytes(contents)
            if (!temporary.renameTo(destination)) {
                destination.writeBytes(contents)
                temporary.delete()
            }
            true
        } catch (_: IOException) {
            false
        }
    }

    @JvmStatic
    fun load(context: Context): ByteArray? {
        val path = File(context.filesDir, FILE_NAME)
        return try {
            if (!path.isFile || path.length() != MIN_LEN.toLong()) null
            else path.readBytes()
        } catch (_: IOException) {
            null
        }
    }

    @JvmStatic
    fun clear(context: Context): Boolean = File(context.filesDir, FILE_NAME).delete()
}
