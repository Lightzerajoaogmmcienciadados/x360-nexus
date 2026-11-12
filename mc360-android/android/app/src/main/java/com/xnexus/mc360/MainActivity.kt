package com.xnexus.mc360

import android.app.Activity
import android.content.Intent
import android.os.Bundle
import android.provider.OpenableColumns
import android.widget.Button
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView

class MainActivity : Activity() {
    private lateinit var status: TextView

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val pane = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(32, 32, 32, 32)
        }
        fun addText(value: String, size: Float) {
            pane.addView(TextView(this).apply { text = value; textSize = size; setPadding(0, 8, 0, 18) })
        }
        addText("MC360 · Android", 27f)
        addText("Pesquisa Rust + Kotlin + Java + C++\nNão executa Minecraft de Xbox 360. Pode analisar o começo de arquivos grandes sem carregá-los inteiros.", 16f)
        pane.addView(Button(this).apply {
            text = "Selecionar arquivo Xbox 360"
            setOnClickListener {
                startActivityForResult(Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
                    addCategory(Intent.CATEGORY_OPENABLE)
                    type = "*/*"
                    addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
                }, 52)
            }
        })
        pane.addView(Button(this).apply {
            text = "Abrir demonstração original"
            setOnClickListener { startActivity(Intent(this@MainActivity, VoxelActivity::class.java)) }
        })
        status = TextView(this).apply { text = "Selecione uma ISO, XEX ou pacote para diagnóstico inicial."; textSize = 16f; setPadding(0, 25, 0, 0) }
        pane.addView(status)
        setContentView(ScrollView(this).apply { addView(pane) })
    }

    @Deprecated("Compatibilidade com Android 7+")
    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        if (requestCode != 52 || resultCode != RESULT_OK) return
        val uri = data?.data ?: return
        status.text = "Analisando cabeçalho..."
        Thread {
            val result = try {
                var filename = uri.lastPathSegment ?: "arquivo"
                var size = -1L
                contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME, OpenableColumns.SIZE), null, null, null)?.use { c ->
                    if (c.moveToFirst()) {
                        val ni = c.getColumnIndex(OpenableColumns.DISPLAY_NAME)
                        val si = c.getColumnIndex(OpenableColumns.SIZE)
                        if (ni >= 0 && !c.isNull(ni)) filename = c.getString(ni)
                        if (si >= 0 && !c.isNull(si)) size = c.getLong(si)
                    }
                }
                val header = ByteArray(4096)
                var n = 0
                (contentResolver.openInputStream(uri) ?: error("Arquivo inacessível")).use { stream ->
                    while (n < header.size) {
                        val count = stream.read(header, n, header.size - n)
                        if (count <= 0) break
                        n += count
                    }
                }
                "Arquivo: " + filename + "\n" + NativeBridge.inspect(header.copyOf(n), filename, size)
            } catch (e: Exception) { "Erro ao analisar: " + (e.message ?: "desconhecido") }
            runOnUiThread { status.text = result }
        }.start()
    }
}
