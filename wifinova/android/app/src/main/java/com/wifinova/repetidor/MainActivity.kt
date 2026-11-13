package com.wifinova.repetidor

import android.app.Activity
import android.content.Intent
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.net.ConnectivityManager
import android.net.NetworkCapabilities
import android.net.wifi.WifiInfo
import android.net.wifi.WifiManager
import android.os.Build
import android.os.Bundle
import android.provider.Settings
import android.widget.Button
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import java.net.InetSocketAddress
import java.net.Socket
import java.util.Locale
import java.util.concurrent.Executors
import kotlin.math.abs

class MainActivity : Activity() {
    private val worker = Executors.newSingleThreadExecutor()
    private val dark = Color.rgb(7, 16, 12)
    private val surface = Color.rgb(17, 39, 27)
    private val green = Color.rgb(90, 235, 145)
    private val light = Color.rgb(234, 255, 239)
    private lateinit var networkStatus: TextView
    private lateinit var testStatus: TextView
    private lateinit var server: EditText
    private lateinit var testButton: Button

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        window.statusBarColor = dark
        window.navigationBarColor = dark
        val root = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(px(18), px(16), px(18), px(28))
            setBackgroundColor(dark)
        }
        heading(root, "WiFiNova v2.0", 27, green)
        line(root, "6 GHz + GAME OPTIMIZER", green)
        line(root, "Diagnóstico da sua rede e acesso às configurações do compartilhamento. Não exige root.", light)

        val wifiBox = panel(root)
        heading(wifiBox, "Repetidor / Hotspot", 20, light)
        line(wifiBox, "Abra o hotspot oficial. Repetição Wi-Fi para Wi-Fi e hotspot na banda de 6 GHz dependem do fabricante e do aparelho.", light)
        button(wifiBox, "Abrir hotspot") {
            try { startActivity(Intent("android.settings.TETHER_SETTINGS")) }
            catch (_: Exception) {
                startActivity(Intent(Settings.ACTION_WIRELESS_SETTINGS))
            }
        }
        button(wifiBox, "Configurações de Wi-Fi") {
            startActivity(Intent(Settings.ACTION_WIFI_SETTINGS))
        }

        val wifiInfo = panel(root)
        heading(wifiInfo, "Wi-Fi 5 GHz / 6 GHz", 20, light)
        networkStatus = line(wifiInfo, "Verificando rede...", light)
        button(wifiInfo, "Atualizar diagnóstico") { updateNetwork() }

        val tests = panel(root)
        heading(tests, "Otimizador para jogos", 20, light)
        line(tests, "Mede 5 conexões TCP na porta 443. Não altera a velocidade da internet nem garante ping em jogos.", light)
        server = EditText(this).apply {
            setText("1.1.1.1")
            setSingleLine(true)
            hint = "Servidor de teste"
            setTextColor(light)
            setHintTextColor(Color.LTGRAY)
        }
        tests.addView(server)
        testButton = button(tests, "Testar conexão") { testTcp() }
        testStatus = line(tests, "Faça o teste para avaliar estabilidade e falhas de conexão.", light)
        line(root, "Wi-Fi 6 GHz exige hardware Wi-Fi 6E ou Wi-Fi 7. Hotspot não pode ser forçado por um aplicativo comum.", Color.LTGRAY)

        setContentView(ScrollView(this).apply {
            isFillViewport = true
            addView(root)
        })
        updateNetwork()
    }

    @Suppress("DEPRECATION")
    private fun updateNetwork() {
        try {
            val wm = applicationContext.getSystemService(WIFI_SERVICE) as WifiManager
            val six = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) wm.is6GHzBandSupported else false
            val five = wm.is5GHzBandSupported
            val cm = getSystemService(CONNECTIVITY_SERVICE) as ConnectivityManager
            val caps = cm.getNetworkCapabilities(cm.activeNetwork)
            val msg = StringBuilder()
            msg.append("5 GHz: ").append(if (five) "suportado" else "não detectado").append("\n")
            msg.append("6 GHz: ").append(if (six) "suportado pelo rádio" else "não detectado").append("\n")
            if (caps == null) {
                msg.append("Conexão atual: indisponível")
            } else if (caps.hasTransport(NetworkCapabilities.TRANSPORT_WIFI)) {
                val info = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q)
                    caps.transportInfo as? WifiInfo else wm.connectionInfo
                val mhz = info?.frequency ?: 0
                val band = when (mhz) {
                    in 5925..7125 -> "6 GHz"
                    in 4900..5900 -> "5 GHz"
                    in 2300..2500 -> "2,4 GHz"
                    else -> "desconhecida"
                }
                msg.append("Conexão atual: Wi-Fi\nBanda: ").append(band)
                if (mhz > 0) msg.append(" (").append(mhz).append(" MHz)")
            } else if (caps.hasTransport(NetworkCapabilities.TRANSPORT_CELLULAR)) {
                msg.append("Conexão atual: dados móveis")
            } else {
                msg.append("Conexão atual: outra rede")
            }
            networkStatus.text = msg.toString()
        } catch (e: Exception) {
            networkStatus.text = "Não foi possível ler informações da rede: " + (e.message ?: "indisponível")
        }
    }

    private fun testTcp() {
        val host = server.text.toString().trim()
        if (!host.matches(Regex("[a-zA-Z0-9.:-]{1,120}")) || host.contains("..") || host.startsWith("-")) {
            testStatus.text = "Digite um domínio ou IP válido."
            return
        }
        testButton.isEnabled = false
        testStatus.text = "Testando 5 conexões TCP..."
        worker.execute {
            val times = mutableListOf<Double>()
            repeat(5) {
                try {
                    val start = System.nanoTime()
                    Socket().use { it.connect(InetSocketAddress(host, 443), 1800) }
                    times.add((System.nanoTime() - start) / 1_000_000.0)
                } catch (_: Exception) { }
            }
            val failures = 5 - times.size
            val avg = if (times.isEmpty()) null else times.average()
            val variation = if (times.size < 2) 0.0 else times.zipWithNext { a, b -> abs(a - b) }.average()
            val report = if (avg == null) {
                "Sem resposta TCP. Verifique internet e servidor."
            } else {
                "Latência média TCP: " + String.format(Locale.US, "%.1f", avg) + " ms\n" +
                    "Variação: " + String.format(Locale.US, "%.1f", variation) + " ms\n" +
                    "Falhas: " + failures + "/5\n\n" +
                    (if (variation > 20) "Conexão oscilando: pause downloads simultâneos.\n" else "Oscilação relativamente baixa.\n") +
                    "Aproxime-se do roteador e prefira 5/6 GHz quando há bom sinal. O ping do jogo pode ser diferente."
            }
            runOnUiThread {
                if (!isDestroyed) {
                    testStatus.text = report
                    testButton.isEnabled = true
                }
            }
        }
    }

    private fun panel(parent: LinearLayout): LinearLayout {
        val p = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(px(14), px(15), px(14), px(14))
            background = GradientDrawable().apply {
                setColor(surface)
                cornerRadius = px(14).toFloat()
                setStroke(px(1), Color.rgb(50, 95, 64))
            }
        }
        parent.addView(p, LinearLayout.LayoutParams(-1, -2).apply { bottomMargin = px(14) })
        return p
    }

    private fun heading(parent: LinearLayout, s: String, size: Int, color: Int) {
        parent.addView(TextView(this).apply {
            text = s
            textSize = size.toFloat()
            setTextColor(color)
            setTypeface(null, Typeface.BOLD)
            setPadding(0, px(5), 0, px(10))
        })
    }

    private fun line(parent: LinearLayout, s: String, color: Int): TextView {
        return TextView(this).apply {
            text = s
            textSize = 14.5f
            setTextColor(color)
            setPadding(0, px(6), 0, px(12))
            parent.addView(this)
        }
    }

    private fun button(parent: LinearLayout, s: String, action: () -> Unit): Button {
        return Button(this).apply {
            text = s
            isAllCaps = false
            setTextColor(dark)
            background = GradientDrawable().apply {
                setColor(green)
                cornerRadius = px(12).toFloat()
            }
            setOnClickListener {
                try { action() }
                catch (e: Exception) { android.widget.Toast.makeText(this@MainActivity, "Erro: " + e.message, android.widget.Toast.LENGTH_SHORT).show() }
            }
            parent.addView(this, LinearLayout.LayoutParams(-1, px(49)).apply { bottomMargin = px(8) })
        }
    }

    private fun px(n: Int) = (n * resources.displayMetrics.density).toInt()
    override fun onDestroy() {
        worker.shutdownNow()
        super.onDestroy()
    }
}
