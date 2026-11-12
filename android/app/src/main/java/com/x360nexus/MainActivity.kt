package com.x360nexus

import android.app.Activity
import android.content.Intent
import android.graphics.Color
import android.net.Uri
import android.os.Bundle
import android.os.SystemClock
import android.provider.OpenableColumns
import android.view.View
import android.widget.ArrayAdapter
import android.widget.Button
import android.widget.CheckBox
import android.widget.LinearLayout
import android.widget.Spinner
import android.widget.TextView
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicBoolean
import org.json.JSONArray
import org.json.JSONObject

/** v4.5 UI and diagnostic execution; no actual commercial Xbox 360 game support. */
class MainActivity : Activity() {
    companion object {
        private const val PICK_ISO=100
        private const val PICK_KEY=102
        private val GREEN=Color.rgb(46,204,113)
        private val RUST_READY:Boolean
        private val NATIVE_READY:Boolean
        init {
            var r=false;var c=false
            try {System.loadLibrary("x360nexus");r=true} catch (_:UnsatisfiedLinkError){}
            try {System.loadLibrary("x360xdvdfs");c=true} catch (_:UnsatisfiedLinkError){}
            RUST_READY=r;NATIVE_READY=c
        }
    }
    private external fun nativeInspectXdvdfs(fd:Int):String
    private external fun nativeLoadDefaultXex(fd:Int):String
    private external fun nativeInspectStagedXex():String
    private external fun nativePrepareStagedXex():String
    private external fun nativePreparedInfo():LongArray?
    private external fun nativeExecutableRanges():LongArray?
    private external fun nativePreparedSnapshot():ByteArray?
    private external fun nativeSetXexKey(bytes:ByteArray):String
    private external fun nativeClearXexKey():String
    private external fun nativeStartSession(image:ByteArray,base:Long,entry:Long,ranges:LongArray):String
    private external fun nativeRunSessionChunk(steps:Int):String
    private external fun nativeStopSession():String
    private external fun nativeSetGpuCapabilities(renderer:String,gles:String,vulkan:String):String
    private external fun nativeSetGraphicsSettings(backend:Int,resolution:Int,turnip:Boolean,
        exact:Boolean,cache:Boolean,vsync:Boolean,fps:Boolean):String

    private data class Game(val uri:Uri,val name:String)
    private val games=mutableListOf<Game>()
    private var selected:Uri?=null
    private val worker=Executors.newSingleThreadExecutor()
    private val continueRunning=AtomicBoolean(false)
    private val prefs by lazy {getSharedPreferences("x360-v45",MODE_PRIVATE)}
    private lateinit var console:TextView
    private lateinit var telemetry:TextView
    private lateinit var gpuText:TextView
    private lateinit var gameList:LinearLayout
    private lateinit var settingsPanel:LinearLayout
    private lateinit var backend:Spinner
    private lateinit var resolution:Spinner
    private lateinit var precision:Spinner
    private lateinit var cache:CheckBox
    private lateinit var vsync:CheckBox
    private lateinit var showFps:CheckBox
    private var busy=false
    private var logBuffer=""
    private var currentGpu="GPU ainda não detectada"

    private fun log(message:String) {
        val stamped="[${SystemClock.uptimeMillis()/1000}s] $message\n"
        logBuffer=(logBuffer+stamped).takeLast(13000)
        console.text=logBuffer
    }
    private fun uiLog(message:String) {runOnUiThread{log(message)}}
    private fun setBusy(on:Boolean){busy=on
        findViewById<Button>(R.id.startGame).isEnabled=!on
        findViewById<Button>(R.id.selectIso).isEnabled=!on
    }
    private fun picker() {
        startActivityForResult(Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
            addCategory(Intent.CATEGORY_OPENABLE);type="*/*"
            addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION)
        },PICK_ISO)
    }
    private fun select(uri:Uri){selected=uri;renderGames()}
    private fun readStoredGames(){
        try {
            val arr=JSONArray(prefs.getString("games","[]"))
            for(i in 0 until arr.length().coerceAtMost(12)){
                val o=arr.getJSONObject(i);games.add(Game(Uri.parse(o.getString("uri")),o.getString("name")))
            }
            selected=games.firstOrNull()?.uri
        }catch (_:Exception){}
    }
    private fun persistGames(){
        val arr=JSONArray()
        games.forEach { arr.put(JSONObject().put("uri",it.uri.toString()).put("name",it.name)) }
        prefs.edit().putString("games",arr.toString()).apply()
    }
    private fun renderGames(){
        gameList.removeAllViews()
        if(games.isEmpty()){
            gameList.addView(TextView(this).apply{text="Nenhuma ISO adicionada";setTextColor(Color.LTGRAY)})
            return
        }
        for(game in games){
            val active=game.uri==selected
            gameList.addView(TextView(this).apply{
                text=(if(active)"●  " else "○  ")+game.name+"\nISO selecionável • Inspeção XDVDFS"
                textSize=15f;setTextColor(if(active) GREEN else Color.WHITE)
                setPadding(10,12,10,12);setOnClickListener{select(game.uri);log("Selecionado: ${game.name}")}
            })
        }
    }
    private fun applySpinner(spinner:Spinner,options:List<String>,saved:Int){
        spinner.adapter=ArrayAdapter(this,android.R.layout.simple_spinner_dropdown_item,options)
        spinner.setSelection(saved.coerceIn(0,options.size-1))
    }
    private fun restoreSettings(){
        applySpinner(backend,listOf("Auto (recomendado)","Vulkan (solicitado)","OpenGL ES (solicitado)"),prefs.getInt("backend",0))
        applySpinner(resolution,listOf("0.5x / 360p","1x / 720p","1.5x / 1080p","2x / 1440p"),prefs.getInt("resolution",1))
        applySpinner(precision,listOf("Rápida","Exata (preferência Mali)"),prefs.getInt("precision",0))
        cache.isChecked=prefs.getBoolean("cache",true)
        vsync.isChecked=prefs.getBoolean("vsync",true)
        showFps.isChecked=prefs.getBoolean("fps",true)
    }
    private fun saveSettings(){
        val be=backend.selectedItemPosition;val scale=resolution.selectedItemPosition
        val exact=precision.selectedItemPosition==1
        prefs.edit().putInt("backend",be).putInt("resolution",scale)
            .putInt("precision",precision.selectedItemPosition)
            .putBoolean("cache",cache.isChecked).putBoolean("vsync",vsync.isChecked)
            .putBoolean("fps",showFps.isChecked).apply()
        val note=if(NATIVE_READY) nativeSetGraphicsSettings(be,scale,false,exact,cache.isChecked,vsync.isChecked,showFps.isChecked)
            else "Biblioteca C++ indisponível"
        findViewById<TextView>(R.id.settingsNotice).text=note
        log(note)
    }
    override fun onCreate(savedInstanceState:Bundle?){
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)
        console=findViewById(R.id.console)
        telemetry=findViewById(R.id.telemetry)
        gpuText=findViewById(R.id.gpuInfo)
        gameList=findViewById(R.id.gameList)
        settingsPanel=findViewById(R.id.settingsPanel)
        backend=findViewById(R.id.backend)
        resolution=findViewById(R.id.resolution)
        precision=findViewById(R.id.precision)
        cache=findViewById(R.id.shaderCache)
        vsync=findViewById(R.id.vsync)
        showFps=findViewById(R.id.fpsToggle)
        restoreSettings();readStoredGames();renderGames()
        findViewById<Button>(R.id.selectIso).setOnClickListener {if(!busy)picker()}
        findViewById<Button>(R.id.loadKey).setOnClickListener {
            startActivityForResult(Intent(Intent.ACTION_OPEN_DOCUMENT).apply{
                addCategory(Intent.CATEGORY_OPENABLE);type="application/octet-stream"
                addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
            },PICK_KEY)
        }
        findViewById<Button>(R.id.startGame).setOnClickListener {startSelected()}
        findViewById<Button>(R.id.stopGame).setOnClickListener {
            continueRunning.set(false)
            if(RUST_READY)worker.execute {uiLog(nativeStopSession())}
        }
        findViewById<Button>(R.id.configButton).setOnClickListener {
            settingsPanel.visibility=if(settingsPanel.visibility==View.VISIBLE)View.GONE else View.VISIBLE
        }
        findViewById<Button>(R.id.saveSettings).setOnClickListener {saveSettings()}
        findViewById<Button>(R.id.consoleToggle).setOnClickListener {
            console.visibility=if(console.visibility==View.VISIBLE)View.GONE else View.VISIBLE
        }
        log("v4.5: Rust JNI=${RUST_READY}, C++ JNI=${NATIVE_READY}. Execução limitada a snapshot de PE32.")
        worker.execute {
            val info=GpuDetector.query(this)
            currentGpu="${info.family}\n${info.renderer}\nOpenGL ES: ${info.version}\nVulkan: ${info.vulkan}"
            val msg=if(NATIVE_READY) nativeSetGpuCapabilities(info.renderer,info.version,info.vulkan) else "JNI C++ indisponível"
            runOnUiThread {gpuText.text=currentGpu;log(msg);saveSettings()}
        }
    }
    private fun startSelected(){
        if(busy || !NATIVE_READY || !RUST_READY){log("Bibliotecas indisponíveis ou execução já iniciada.");return}
        val iso=selected?:run{log("Adicione e selecione uma ISO primeiro.");return}
        setBusy(true);continueRunning.set(true)
        worker.execute {
            try {
                contentResolver.openFileDescriptor(iso,"r")?.use { fd ->
                    uiLog(nativeLoadDefaultXex(fd.fd))
                }?:throw IllegalStateException("Não foi possível abrir a ISO")
                if(!continueRunning.get())return@execute
                uiLog(nativeInspectStagedXex())
                val preparation=nativePrepareStagedXex()
                uiLog(preparation)
                val info=nativePreparedInfo()?:throw IllegalStateException("PE não preparado: verifique AES/LZX e imports")
                val ranges=nativeExecutableRanges()?:throw IllegalStateException("Secoes executaveis inválidas")
                val snapshot=nativePreparedSnapshot()?:throw IllegalStateException("Imagem maior que 16 MiB ou RAM indisponível para o teste")
                val start=try{nativeStartSession(snapshot,info[0],info[1],ranges)}finally{snapshot.fill(0)}
                uiLog(start)
                if(!start.startsWith("OK|"))return@execute
                var lastHud=0L
                var lastReport=""
                while(continueRunning.get()){
                    lastReport=nativeRunSessionChunk(8000)
                    val now=SystemClock.uptimeMillis()
                    if(now-lastHud>=350 || !lastReport.startsWith("RUNNING|")){
                        val payload=lastReport
                        runOnUiThread {
                            val ips=Regex("IPS=([0-9]+)").find(payload)?.groupValues?.get(1)?:"0"
                            telemetry.text="RAM snapshot: ${info[2]/1024/1024} MiB | IPS: $ips | FPS: N/D (sem Xenos)"
                            log(payload)
                        }
                        lastHud=now
                    }
                    if(!lastReport.startsWith("RUNNING|"))break
                }
                if(!continueRunning.get())uiLog(nativeStopSession())
            }catch(e:Throwable){uiLog("PARADA: ${e.message}")}
            finally{continueRunning.set(false);runOnUiThread{setBusy(false)}}
        }
    }
    @Deprecated("Legacy Storage Access Framework callback")
    override fun onActivityResult(requestCode:Int,resultCode:Int,data:Intent?){
        super.onActivityResult(requestCode,resultCode,data)
        if(resultCode!=RESULT_OK)return
        val uri=data?.data?:return
        if(requestCode==PICK_ISO){
            try {contentResolver.takePersistableUriPermission(uri,Intent.FLAG_GRANT_READ_URI_PERMISSION)}catch(_:SecurityException){}
            var name=uri.lastPathSegment?:"ISO"
            try {contentResolver.query(uri,null,null,null,null)?.use{cur->
                if(cur.moveToFirst()){
                    val index=cur.getColumnIndex(OpenableColumns.DISPLAY_NAME)
                    if(index>=0)name=cur.getString(index)
                }
            }}catch(_:Exception){}
            games.removeAll{it.uri==uri};games.add(0,Game(uri,name))
            while(games.size>12)games.removeAt(games.lastIndex)
            selected=uri;persistGames();renderGames();log("ISO adicionada: $name")
        } else if(requestCode==PICK_KEY){
            if(busy)return
            worker.execute{
                val buf=ByteArray(17)
                val message=try{
                    val amount=contentResolver.openInputStream(uri)?.use{input->
                        var offset=0
                        while(offset<17){val n=input.read(buf,offset,17-offset);if(n<0)break;if(n>0)offset+=n}
                        offset
                    }?:0
                    if(amount!=16) "keys.bin deve ter 16 bytes; leitura rejeitada" else {
                        val key=buf.copyOfRange(0,16)
                        try{nativeSetXexKey(key)}finally{key.fill(0)}
                    }
                }catch(e:Throwable){"Falha lendo chave: ${e.message}"}finally{buf.fill(0)}
                uiLog(message)
            }
        }
    }
    override fun onDestroy(){
        continueRunning.set(false)
        worker.shutdownNow()
        if(NATIVE_READY)nativeClearXexKey()
        super.onDestroy()
    }
}