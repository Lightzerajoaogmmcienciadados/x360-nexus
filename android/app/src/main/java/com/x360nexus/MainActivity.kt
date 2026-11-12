package com.x360nexus

import android.app.Activity
import android.content.Intent
import android.database.Cursor
import android.graphics.Color
import android.net.Uri
import android.os.Bundle
import android.os.ParcelFileDescriptor
import android.provider.OpenableColumns
import android.widget.Button
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import java.util.concurrent.Executors

/** X360 Nexus v4.3: XDVDFS, XEX2 metadata, PE32 mapping and bounded PPC trace.
 *  Does not decrypt retail titles, implement Xenos, or run commercial games.
 */
class MainActivity : Activity() {
    companion object {
        private const val PICK_ISO = 100
        private const val SAVE_XEX = 101
        private val GREEN = Color.rgb(30, 210, 95)
        private val RUST_READY: Boolean
        private val XDVDFS_READY: Boolean
        init {
            var rust = false
            var xdvdfs = false
            try { System.loadLibrary("x360nexus"); rust = true } catch (_: UnsatisfiedLinkError) { }
            try { System.loadLibrary("x360xdvdfs"); xdvdfs = true } catch (_: UnsatisfiedLinkError) { }
            RUST_READY = rust
            XDVDFS_READY = xdvdfs
        }
    }

    private external fun nativeSelfTest(): String
    private external fun nativeInspectXdvdfs(fd: Int): String
    private external fun nativeLoadDefaultXex(fd: Int): String
    private external fun nativeSaveDefaultXex(inputFd: Int, outputFd: Int): String
    private external fun nativeGuestRamStatus(): String
    private external fun nativeInspectStagedXex(): String
    private external fun nativePrepareStagedXex(): String
    private external fun nativePreparedEntryPoint(): Long
    private external fun nativeFetchCodeWindow(address: Long, length: Int): ByteArray?
    private external fun nativeTracePpc(bytes: ByteArray, guestPc: Long, steps: Int): String

    private val worker = Executors.newSingleThreadExecutor()
    private val buttons = mutableListOf<Button>()
    private lateinit var result: TextView
    private var selectedIso: Uri? = null
    private var busy = false

    private fun text(value: String, sp: Float, color: Int): TextView = TextView(this).apply {
        text = value
        textSize = sp
        setTextColor(color)
        setPadding(8, 12, 8, 12)
        setTextIsSelectable(true)
    }
    private fun button(title: String, root: LinearLayout, action: () -> Unit) {
        val b = Button(this).apply { text = title; isAllCaps = false; setOnClickListener { action() } }
        buttons.add(b)
        root.addView(b)
    }
    private fun show(message: String) { result.text = message }
    private fun busy(value: Boolean) {
        busy = value
        buttons.forEach { it.isEnabled = !value }
    }
    private fun withIso(title: String, action: (Int) -> String) {
        if (busy) return
        if (!XDVDFS_READY) { show("Biblioteca C++/JNI nao carregada"); return }
        val uri = selectedIso ?: run { show("Selecione uma ISO antes."); return }
        busy(true)
        show("$title\nLendo ISO selecionada...")
        worker.execute {
            val response = try {
                contentResolver.openFileDescriptor(uri, "r")?.use { action(it.fd) }
                    ?: "Nao foi possivel abrir a ISO"
            } catch (error: Throwable) {
                "Falha: ${error.message}"
            }
            runOnUiThread { busy(false); show(response) }
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val scroll = ScrollView(this)
        val root = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(24, 36, 24, 36)
            setBackgroundColor(Color.rgb(7, 15, 20))
        }
        root.addView(text("PROJECT XENON", 32f, GREEN))
        root.addView(text("4.3: GALAXY | X360 Nexus Android\nXDVDFS + XEX2 + PowerPC Trace", 17f, Color.WHITE))
        root.addView(text("PE32 • CPU diagnostica • HLE experimental", 17f, GREEN))

        button("1. SELECIONAR ISO", root) {
            startActivityForResult(Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
                addCategory(Intent.CATEGORY_OPENABLE)
                type = "*/*"
                addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION)
            }, PICK_ISO)
        }
        button("2. ANALISAR ISO E LISTAR ARQUIVOS", root) {
            withIso("Analisando XDVDFS...") { nativeInspectXdvdfs(it) }
        }
        button("3. CARREGAR default.xex NA RAM EMULADA", root) {
            withIso("Copiando XEX bruto para staging...") { nativeLoadDefaultXex(it) }
        }
        button("4. EXTRAIR default.xex PARA ARQUIVO", root) {
            if (busy || selectedIso == null) { show("Selecione uma ISO antes."); return@button }
            startActivityForResult(Intent(Intent.ACTION_CREATE_DOCUMENT).apply {
                addCategory(Intent.CATEGORY_OPENABLE)
                type = "application/octet-stream"
                putExtra(Intent.EXTRA_TITLE, "default.xex")
            }, SAVE_XEX)
        }
        button("ANALISAR CABECALHOS XEX2", root) {
            if (!XDVDFS_READY) { show("Nucleo C++ indisponivel"); return@button }
            try { show(nativeInspectStagedXex()) } catch (e: Throwable) { show("Erro XEX: ${e.message}") }
        }
        button("5. EXECUTAR XEX NA CPU (TRACE LIMITADO)", root) {
            if (busy) return@button
            if (!XDVDFS_READY || !RUST_READY) { show("JNI C++ ou Rust indisponivel"); return@button }
            busy(true)
            show("Preparando imagem PE32 e CPU...")
            worker.execute {
                val message = try {
                    val status = nativePrepareStagedXex()
                    val entry = nativePreparedEntryPoint()
                    if (entry == 0L) {
                        "$status\n\nExecucao impedida: sem PE pronto / sem chave / sem LZX."
                    } else {
                        // No more than 256 bytes are copied to the bounded Rust interpreter.
                        val code = listOf(256, 128, 64, 32, 16, 8, 4)
                            .firstNotNullOfOrNull { nativeFetchCodeWindow(entry, it) }
                        if (code == null) "$status\nEntry Point nao acessivel em secao executavel."
                        else "$status\n\n" + nativeTracePpc(code, entry, 32)
                    }
                } catch (error: Throwable) {
                    "Falha ao preparar/interpretar XEX: ${error.message}"
                }
                runOnUiThread { busy(false); show(message) }
            }
        }
        button("CONSULTAR MEMORIA RAM EMULADA", root) {
            if (XDVDFS_READY) show(nativeGuestRamStatus()) else show("Biblioteca C++ indisponivel")
        }
        button("TESTAR CPU POWERPC + KERNEL (RUST)", root) {
            if (RUST_READY) show(nativeSelfTest()) else show("Nucleo Rust indisponivel")
        }

        result = text("X360 Nexus v4.3\n\nRust JNI: " + (if (RUST_READY) "OK" else "Indisponivel") +
            "\nC++ JNI: " + (if (XDVDFS_READY) "OK" else "Indisponivel") +
            "\n\nPara testar: selecione ISO -> carregue default.xex -> analise XEX -> trace.\n" +
            "XEX Retail criptografado/LZX sera identificado, mas nao executado.\n" +
            "Stubs xboxkrnl/xam ainda nao sao servicos reais.", 16f, Color.LTGRAY)
        root.addView(result)
        scroll.addView(root)
        setContentView(scroll)
    }

    @Deprecated("Uses Android Storage Access Framework legacy activity result")
    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        if (resultCode != RESULT_OK) return
        val uri = data?.data ?: return
        if (requestCode == PICK_ISO) {
            selectedIso = uri
            try { contentResolver.takePersistableUriPermission(uri, Intent.FLAG_GRANT_READ_URI_PERMISSION) }
            catch (_: SecurityException) { }
            var name = uri.lastPathSegment ?: "ISO"
            var size = -1L
            try {
                contentResolver.query(uri, null, null, null, null)?.use { c: Cursor ->
                    if (c.moveToFirst()) {
                        val n = c.getColumnIndex(OpenableColumns.DISPLAY_NAME)
                        if (n >= 0) name = c.getString(n)
                        val s = c.getColumnIndex(OpenableColumns.SIZE)
                        if (s >= 0 && !c.isNull(s)) size = c.getLong(s)
                    }
                }
            } catch (_: Exception) { }
            show("ISO selecionada: $name\nTamanho: $size bytes\n\nToque em 2 para analisar XDVDFS.")
        } else if (requestCode == SAVE_XEX) {
            val source = selectedIso ?: run { show("Selecione uma ISO"); return }
            if (!XDVDFS_READY || busy) return
            busy(true)
            show("Extraindo default.xex...")
            worker.execute {
                val message = try {
                    contentResolver.openFileDescriptor(source, "r")?.use { src: ParcelFileDescriptor ->
                        contentResolver.openFileDescriptor(uri, "w")?.use { dst: ParcelFileDescriptor ->
                            nativeSaveDefaultXex(src.fd, dst.fd)
                        }
                    } ?: "Nao foi possivel abrir os arquivos"
                } catch (e: Throwable) { "Erro de extracao: ${e.message}" }
                runOnUiThread { busy(false); show(message) }
            }
        }
    }
    override fun onDestroy() {
        worker.shutdown()
        super.onDestroy()
    }
}