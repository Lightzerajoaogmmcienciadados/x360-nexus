package com.x360nexus

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/** Private, authenticated 16-byte key persistence. AndroidKeyStore holds the wrapping key.
 * SharedPreferences contain only AES-GCM ciphertext, never plaintext key/hex text.
 * No backup: AndroidManifest has android:allowBackup=false.
 */
object SecretKeyStore {
    private const val ALIAS = "x360.galaxy.user.xex.v52"
    private const val PREF = "x360_private_keys"
    private const val KEY = "wrapped_user_key"

    private fun key(): SecretKey {
        val store = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        (store.getKey(ALIAS, null) as? SecretKey)?.let { return it }
        val generator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore")
        generator.init(KeyGenParameterSpec.Builder(ALIAS,
            KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
            .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
            .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
            .setKeySize(256).build())
        return generator.generateKey()
    }

    fun save(context: Context, raw: ByteArray) {
        require(raw.size == 16) { "Uma chave deve conter 16 bytes" }
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.ENCRYPT_MODE, key())
        val encrypted = cipher.doFinal(raw)
        val packed = cipher.iv + encrypted
        context.getSharedPreferences(PREF, Context.MODE_PRIVATE)
            .edit().putString(KEY, Base64.encodeToString(packed, Base64.NO_WRAP)).apply()
        packed.fill(0)
    }

    fun restore(context: Context): ByteArray? {
        val encoded = context.getSharedPreferences(PREF, Context.MODE_PRIVATE)
            .getString(KEY, null) ?: return null
        val payload = Base64.decode(encoded, Base64.NO_WRAP)
        require(payload.size == 12 + 16 + 16) { "Dados de chave inválidos" }
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.DECRYPT_MODE, key(), GCMParameterSpec(128, payload.copyOfRange(0, 12)))
        val raw = cipher.doFinal(payload, 12, payload.size - 12)
        payload.fill(0)
        require(raw.size == 16) { "Chave recuperada inválida" }
        return raw
    }

    fun clear(context: Context) {
        context.getSharedPreferences(PREF, Context.MODE_PRIVATE).edit().remove(KEY).apply()
    }
}