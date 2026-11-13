package com.x360nexus

import android.app.Activity
import android.content.pm.ActivityInfo
import android.widget.FrameLayout
import android.view.ViewGroup
import android.view.LayoutInflater
import android.app.AlertDialog
import android.text.InputType
import android.view.WindowManager
import android.widget.EditText
import android.widget.Toast
import android.content.Intent
import android.graphics.Color
import android.net.Uri
import android.os.Bundle
import android.os.SystemClock
import android.provider.OpenableColumns
import android.view.View
import android.view.SurfaceHolder
import android.view.SurfaceView
import android.view.Surface
import android.view.MotionEvent
import android.view.KeyEvent
import android.os.Handler
import android.os.Looper
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

/** v5.1 Android UI, sparse guest RAM session and diagnostic GLES PM4 processing. */
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
    private external fun nativePreparedSnapshot():ByteArray? // legacy diagnostic only
    private external fun nativePreparedChunk(address:Long,length:Int):ByteArray?
    private external fun nativeStartSessionPaged(base:Long,entry:Long,size:Int,ranges:LongArray):String
    private external fun nativeUploadSessionChunk(address:Long,data:ByteArray):String
    private external fun nativeLaunchSession():String
    private external fun nativePollSession():String
    private external fun nativeImportThunks():LongArray?
    private external fun nativeImportVariables():LongArray?
    private external fun nativeConfigureImportVariables(entries:LongArray):String
    private external fun nativeConfigureImportThunks(entries:LongArray):String
    private external fun nativeSetXexKey(bytes:ByteArray):String
    private external fun nativeClearXexKey():String
    private external fun nativeStartSession(image:ByteArray,base:Long,entry:Long,ranges:LongArray):String
    private external fun nativeRunSessionChunk(steps:Int):String
    private external fun nativeStopSession():String
    private external fun nativeSetVideoSurface(surface:Surface?):String
    private external fun nativeRenderVideoFrame():String
    private external fun nativeVideoStats():String
    private external fun nativeSubmitDemoPm4():String
    private external fun nativeProcessPm4(address:Long, dwords:Int):String
    private external fun nativeSubmitPm4Bytes(packet:ByteArray):String
    private external fun nativeReadSessionMemory(address:Long,length:Int):ByteArray?
    private external fun nativeSetButton(mask:Int, pressed:Boolean)
    private external fun nativeSetAxis(axis:Int, value:Int)
    private external fun nativeSetGpuCapabilities(renderer:String,gles:String,vulkan:String):String
    private external fun nativeSetGraphicsSettings(backend:Int,resolution:Int,turnip:Boolean,
        exact:Boolean,cache:Boolean,vsync:Boolean,fps:Boolean):String

    private data class Game(val uri:Uri,val name:String)
    private val games=mutableListOf<Game>()
    private var selected:Uri?=null
    private val worker=Executors.newSingleThreadExecutor()
    private val videoWorker=Executors.newSingleThreadExecutor()
    private val videoHandler=Handler(Looper.getMainLooper())
    private val videoQueued=AtomicBoolean(false)
    @Volatile private var videoActive=false
    private var videoTickCount=0
    private val videoTick=object:Runnable {
        override fun run() {
            if(!videoActive)return
            if(videoQueued.compareAndSet(false,true)){
                videoWorker.execute {
                    try {
                        val msg=if(NATIVE_READY)nativeRenderVideoFrame() else "C++ JNI indisponivel"
                        if(msg.contains("failed") || msg.contains("indisponivel"))uiLog(msg)
                        videoTickCount++
                        if(videoTickCount%45==0 && NATIVE_READY){
                            val stat=nativeVideoStats()
                            runOnUiThread {findViewById<TextView>(R.id.videoStats).text=stat;gameFps?.text=stat.take(155)}
                        }
                    }catch(t:Throwable){uiLog("GPU EGL: ${t.message}")}
                    finally {videoQueued.set(false)}
                }
            }
            videoHandler.postDelayed(this,33L)
        }
    }
    private val continueRunning=AtomicBoolean(false)
    private val prefs by lazy {getSharedPreferences("x360-v50",MODE_PRIVATE)}
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
    @Volatile private var preparedUri:Uri?=null
    private var logBuffer=""
    private var currentGpu="GPU ainda não detectada"
    private var lastSwap=0L
    private var keyReady=false
    private var gameOverlay:View?=null
    private var gameFps:TextView?=null
    private var gameConsole:TextView?=null

    private fun log(message:String) {
        val stamped="[${SystemClock.uptimeMillis()/1000}s] $message\n"
        logBuffer=(logBuffer+stamped).takeLast(13000)
        console.text=logBuffer
        gameConsole?.text=logBuffer
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
    private fun select(uri:Uri){selected=uri;preparedUri=null;renderGames();launchOneClick()}
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
        restoreXexKey()
        setupVideo()
        setupPad()
        applySafeInsets()
        findViewById<Button>(R.id.pm4Demo).setOnClickListener {
            if(NATIVE_READY)log(nativeSubmitDemoPm4()) else log("PM4 JNI indisponivel")
        }
        findViewById<Button>(R.id.selectIso).setOnClickListener {if(!busy)picker()}
        findViewById<Button>(R.id.loadKey).setOnClickListener { showKeyDialog() }
        findViewById<Button>(R.id.startGame).setOnClickListener {launchOneClick()}
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
        log("v5.5: Rust JNI=${RUST_READY}, C++ JNI=${NATIVE_READY}. XEX AES/Basic e imports verificados; GPU Xenos ainda incompleta.")
        worker.execute {
            val info=GpuDetector.query(this)
            currentGpu="${info.family}\n${info.renderer}\nOpenGL ES: ${info.version}\nVulkan: ${info.vulkan}"
            val msg=if(NATIVE_READY) nativeSetGpuCapabilities(info.renderer,info.version,info.vulkan) else "JNI C++ indisponível"
            runOnUiThread {gpuText.text=currentGpu;log(msg);saveSettings()}
        }
        // Restored library entries are not launched until selected by the user.
    }
    private fun restoreXexKey() {
        if(!NATIVE_READY)return
        try {
            val raw=SecretKeyStore.restore(this) ?: return
            try {
                val result=nativeSetXexKey(raw)
                keyReady=result.contains("carregada")
                log(if(keyReady) "Chave salva recuperada do AndroidKeyStore" else result)
            }
            finally {raw.fill(0)}
        }catch(e:Exception){SecretKeyStore.clear(this);log("Chave salva não pôde ser recuperada; importe novamente")}
    }

    private fun showKeyDialog() {
        val input=EditText(this).apply {
            hint="32 caracteres hexadecimais (128 bits)"
            inputType=InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_PASSWORD
            isSingleLine=true
            setPadding(24,20,24,20)
        }
        val dlg=AlertDialog.Builder(this)
            .setTitle("Chave XEX externa")
            .setMessage("Cole 32 dígitos hexadecimais. A chave será cifrada com AndroidKeyStore; nenhum valor será registrado no log.")
            .setView(input)
            .setPositiveButton("APLICAR CHAVE HEX",null)
            .setNeutralButton("IMPORTAR KEYS.BIN") { _, _ ->
                startActivityForResult(Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
                    addCategory(Intent.CATEGORY_OPENABLE)
                    type="application/octet-stream"
                    addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
                },PICK_KEY)
            }
            .setNegativeButton("CANCELAR",null)
            .create()
        dlg.setOnShowListener {
            dlg.window?.addFlags(WindowManager.LayoutParams.FLAG_SECURE)
            dlg.getButton(AlertDialog.BUTTON_POSITIVE).setOnClickListener {
                val hex=input.text.toString().trim()
                if(!Regex("[0-9a-fA-F]{32}").matches(hex)) {
                    input.error="Use exatamente 32 caracteres de 0-9 e A-F"
                    return@setOnClickListener
                }
                val raw=ByteArray(16) {i -> hex.substring(i*2,i*2+2).toInt(16).toByte()}
                input.text.clear()
                try {
                    val result=nativeSetXexKey(raw)
                    preparedUri=null
                    if(result.contains("carregada")) {
                        SecretKeyStore.save(this,raw)
                        keyReady=true
                        log("Chave hexadecimal aplicada (16 bytes) e salva cifrada")
                        dlg.dismiss()
                        if(selected!=null)launchOneClick()
                    }else {log(result)}
                }catch(_:Exception){log("Falha aplicando chave; dados não foram exibidos")}
                finally {raw.fill(0)}
            }
        }
        dlg.show()
    }

    /** Attempts XEX preparation immediately after ISO or key selection.
     *  No game code is launched by this operation. */
    private fun prepareIsoOnWorker(iso:Uri):LongArray {
        val load=contentResolver.openFileDescriptor(iso,"r")?.use {fd ->
            nativeLoadDefaultXex(fd.fd)
        }?:throw IllegalStateException("Nao foi possivel abrir a ISO")
        if(!load.startsWith("default.xex armazenado"))throw IllegalStateException(load)
        uiLog(load)
        uiLog(nativeInspectStagedXex())
        val result=nativePrepareStagedXex()
        uiLog(result)
        val info=nativePreparedInfo()
            ?:throw IllegalStateException("PE nao preparado: confira a chave AES, Basic/LZX e a integridade do XEX")
        preparedUri=iso
        return info
    }
    private fun schedulePreparation(trigger:String){
        if(!NATIVE_READY || busy)return
        val iso=selected?:return
        preparedUri=null
        setBusy(true)
        worker.execute {
            try {
                uiLog("Preparacao XEX automatica: $trigger")
                prepareIsoOnWorker(iso)
                uiLog("PE preparado = true. Toque INICIAR para iniciar a CPU e diagnosticar imports.")
            }catch(e:Throwable){preparedUri=null;uiLog("PREPARACAO XEX: ${e.message}")}
            finally{runOnUiThread{setBusy(false)}}
        }
    }
    /** One touch chooses the ISO and opens a host game surface immediately.
     * The CPU/PM4 pipeline may still stop on an unsupported kernel import. */
    private fun launchOneClick(){
        if(busy || continueRunning.get())return
        if(selected==null){log("Selecione uma ISO primeiro");return}
        if(!keyReady){
            log("Chave XEX externa ainda nao carregada. Jogos sem criptografia podem usar modo manual.")
            showKeyDialog();return
        }
        enterGameMode()
        startSelected()
    }
    private fun setImmersive(enabled:Boolean){
        @Suppress("DEPRECATION")
        window.decorView.systemUiVisibility=if(enabled){
            View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY or View.SYSTEM_UI_FLAG_FULLSCREEN or
                View.SYSTEM_UI_FLAG_HIDE_NAVIGATION or View.SYSTEM_UI_FLAG_LAYOUT_STABLE or
                View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN or View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION
        }else{View.SYSTEM_UI_FLAG_LAYOUT_STABLE}
        if(enabled)window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        else window.clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
    }
    private fun gameTouch(id:Int,mask:Int){
        gameOverlay?.findViewById<Button>(id)?.setOnTouchListener {v,event->
            when(event.actionMasked){
                MotionEvent.ACTION_DOWN -> {if(RUST_READY)nativeSetButton(mask,true);v.alpha=0.95f;true}
                MotionEvent.ACTION_UP,MotionEvent.ACTION_CANCEL -> {if(RUST_READY)nativeSetButton(mask,false);v.alpha=0.55f;true}
                else->true
            }
        }
    }
    private fun gameTrigger(id:Int,axis:Int){
        gameOverlay?.findViewById<Button>(id)?.setOnTouchListener {v,event->
            when(event.actionMasked){
                MotionEvent.ACTION_DOWN->{if(RUST_READY)nativeSetAxis(axis,255);v.alpha=0.95f;true}
                MotionEvent.ACTION_UP,MotionEvent.ACTION_CANCEL->{if(RUST_READY)nativeSetAxis(axis,0);v.alpha=0.55f;true}
                else->true
            }
        }
    }
    private fun enterGameMode(){
        if(gameOverlay!=null)return
        requestedOrientation=ActivityInfo.SCREEN_ORIENTATION_SENSOR_LANDSCAPE
        setImmersive(true)
        val root=findViewById<FrameLayout>(android.R.id.content)
        val overlay=LayoutInflater.from(this).inflate(R.layout.activity_game,root,false)
        root.addView(overlay,ViewGroup.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT,ViewGroup.LayoutParams.MATCH_PARENT))
        gameOverlay=overlay
        gameFps=overlay.findViewById(R.id.gameFps)
        gameConsole=overlay.findViewById(R.id.gameConsole)
        gameConsole?.text=logBuffer
        val panel=overlay.findViewById<LinearLayout>(R.id.gameConsolePanel)
        overlay.findViewById<Button>(R.id.gameMenu).setOnClickListener {
            panel.visibility=if(panel.visibility==View.VISIBLE)View.GONE else View.VISIBLE
            gameConsole?.text=logBuffer
        }
        overlay.findViewById<Button>(R.id.gameCloseLog).setOnClickListener{panel.visibility=View.GONE}
        overlay.findViewById<Button>(R.id.gameExit).setOnClickListener{exitGameMode(true)}
        for((id,mask) in listOf(R.id.gameUp to 0x0001,R.id.gameDown to 0x0002,
            R.id.gameLeft to 0x0004,R.id.gameRight to 0x0008,
            R.id.gameStart to 0x0010,R.id.gameBack to 0x0020,
            R.id.gameLB to 0x0100,R.id.gameRB to 0x0200,
            R.id.gameA to 0x1000,R.id.gameB to 0x2000,
            R.id.gameX to 0x4000,R.id.gameY to 0x8000))gameTouch(id,mask)
        gameTrigger(R.id.gameLT,4);gameTrigger(R.id.gameRT,5)
        overlay.findViewById<AnalogPad>(R.id.gameLeftStick).onAxis={x,y->
            if(RUST_READY){nativeSetAxis(0,x);nativeSetAxis(1,y)}
        }
        overlay.findViewById<AnalogPad>(R.id.gameRightStick).onAxis={x,y->
            if(RUST_READY){nativeSetAxis(2,x);nativeSetAxis(3,y)}
        }
        val surface=overlay.findViewById<SurfaceView>(R.id.gameSurface)
        surface.holder.addCallback(object:SurfaceHolder.Callback{
            override fun surfaceCreated(h:SurfaceHolder){
                videoActive=true
                videoWorker.execute{if(NATIVE_READY)nativeSetVideoSurface(h.surface)}
                videoHandler.removeCallbacks(videoTick);videoHandler.post(videoTick)
            }
            override fun surfaceChanged(h:SurfaceHolder,format:Int,width:Int,height:Int){}
            override fun surfaceDestroyed(h:SurfaceHolder){
                videoWorker.execute{if(NATIVE_READY)nativeSetVideoSurface(null)}
            }
        })
        uiLog("Tela paisagem iniciada. Renderizacao GLES atual ainda e diagnostica, nao Minecraft.")
    }
    private fun exitGameMode(stop:Boolean){
        val overlay=gameOverlay?:return
        gameOverlay=null;gameFps=null;gameConsole=null
        if(stop){continueRunning.set(false);if(RUST_READY)worker.execute{uiLog(nativeStopSession())}}
        (overlay.parent as? ViewGroup)?.removeView(overlay)
        requestedOrientation=ActivityInfo.SCREEN_ORIENTATION_UNSPECIFIED
        setImmersive(false)
        val mainSurface=findViewById<SurfaceView>(R.id.videoSurface)
        if(mainSurface.holder.surface.isValid){
            videoWorker.execute{if(NATIVE_READY)nativeSetVideoSurface(mainSurface.holder.surface)}
        }
    }
    @Deprecated("Use controls in the game overlay")
    override fun onBackPressed(){
        if(gameOverlay!=null){exitGameMode(true)}else super.onBackPressed()
    }
    private fun startSelected(){
        lastSwap=0
        if(busy || !NATIVE_READY || !RUST_READY){log("Bibliotecas indisponíveis ou execução já iniciada.");return}
        val iso=selected?:run{log("Adicione e selecione uma ISO primeiro.");return}
        setBusy(true);continueRunning.set(true)
        worker.execute {
            try {
                val info=if(preparedUri==iso)nativePreparedInfo()?:prepareIsoOnWorker(iso)
                    else prepareIsoOnWorker(iso)
                if(!continueRunning.get())return@execute
                val ranges=nativeExecutableRanges()?:throw IllegalStateException("Secoes executaveis inválidas")
                // Transfer the prepared PE in 64-KiB blocks. Sparse RAM is owned
                // by the Rust CPU; Java never holds the whole PE as a byte array.
                val size=info[2].toInt()
                if(size<=0 || size>128*1024*1024)throw IllegalStateException("PE excede limite de seguranca (128 MiB)")
                val start=nativeStartSessionPaged(info[0],info[1],size,ranges)
                uiLog(start)
                if(!start.startsWith("OK|"))return@execute
                var sent=0
                while(sent<size && continueRunning.get()){
                    val count=minOf(65536,size-sent)
                    val address=info[0]+sent
                    val bytes=nativePreparedChunk(address,count)
                        ?:throw IllegalStateException("Falha transferindo PE @0x${address.toString(16)}")
                    val msg=try{nativeUploadSessionChunk(address,bytes)}finally{bytes.fill(0)}
                    if(!msg.startsWith("OK|"))throw IllegalStateException(msg)
                    sent+=count
                    if(sent%(4*1024*1024)==0)uiLog("PE transferido: ${sent/1024/1024} MiB / ${size/1024/1024} MiB")
                }
                if(!continueRunning.get())return@execute
                val variableCatalog=nativeImportVariables()
                if(variableCatalog!=null){
                    val variableStatus=nativeConfigureImportVariables(variableCatalog)
                    uiLog("VARIABLE IMPORTS: $variableStatus")
                    if(!variableStatus.startsWith("OK|"))throw IllegalStateException(variableStatus)
                }else uiLog("VARIABLE IMPORTS: lista indisponivel")
                val catalog=nativeImportThunks()
                if(catalog!=null){
                    val configured=nativeConfigureImportThunks(catalog)
                    uiLog("IMPORTS: $configured")
                }else uiLog("IMPORTS: catalogo indisponivel")
                val launched=nativeLaunchSession()
                uiLog(launched)
                if(!launched.startsWith("OK|"))return@execute
                var last=""
                while(continueRunning.get()){
                    Thread.sleep(350)
                    last=nativePollSession()
                    val status=last
                    runOnUiThread {
                        val ips=Regex("IPS=([0-9]+)").find(status)?.groupValues?.get(1)?:"0"
                        telemetry.text="RAM virtual: 512 MiB | PE: ${size/1024/1024} MiB | IPS: $ips | PM4: parcial"
                        log(status)
                    }
                    val state=Regex("GPU_RING=0x([0-9A-Fa-f]+); GPU_DWORDS=([0-9]+); SWAPS=([0-9]+)").find(status)
                    if(state!=null && NATIVE_READY){
                        val swaps=state.groupValues[3].toLongOrNull()?:0L
                        val n=state.groupValues[2].toIntOrNull()?:0
                        if(swaps!=lastSwap){
                            lastSwap=swaps
                            if(n in 1..4096){
                                val address=state.groupValues[1].toLong(16)
                                val raw=nativeReadSessionMemory(address,n*4)
                                if(raw!=null){
                                    val msg=try{nativeSubmitPm4Bytes(raw)}finally{raw.fill(0)}
                                    uiLog("PM4/RAM: $msg")
                                }
                            }
                        }
                    }
                    if(!last.startsWith("RUNNING|")){
                        runOnUiThread {
                            gameConsole?.text=logBuffer
                            gameOverlay?.findViewById<LinearLayout>(R.id.gameConsolePanel)?.visibility=View.VISIBLE
                        }
                        break
                    }
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
            selected=uri;preparedUri=null;persistGames();renderGames();log("ISO adicionada: $name")
            launchOneClick()
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
                        try {
                            val response=nativeSetXexKey(key)
                            preparedUri=null
                            if(response.contains("carregada")) {
                                SecretKeyStore.save(this, key)
                                keyReady=true
                                "Chave carregada e armazenada cifrada no AndroidKeyStore"
                            } else response
                        } finally { key.fill(0) }
                    }
                }catch(e:Throwable){"Falha lendo chave: ${e.message}"}finally{buf.fill(0)}
                uiLog(message)
                if(message.contains("armazenada cifrada"))runOnUiThread {
                    launchOneClick()
                }
            }
        }
    }
    /** Android 15/16 edge-to-edge: compensate status and navigation bars once. */
    private fun applySafeInsets(){
        val root=findViewById<android.widget.ScrollView>(R.id.rootScroll)
        val density=resources.displayMetrics.density
        val extraTop=(12*density).toInt()
        val extraBottom=(8*density).toInt()
        root.setOnApplyWindowInsetsListener {view,insets ->
            val top:Int
            val bottom:Int
            if(android.os.Build.VERSION.SDK_INT>=30){
                val bars=insets.getInsets(android.view.WindowInsets.Type.systemBars())
                top=bars.top;bottom=bars.bottom
            }else{
                @Suppress("DEPRECATION")
                top=insets.systemWindowInsetTop
                @Suppress("DEPRECATION")
                bottom=insets.systemWindowInsetBottom
            }
            view.setPadding(0,top+extraTop,0,bottom+extraBottom)
            insets
        }
        root.requestApplyInsets()
    }
    private fun setupVideo(){
        val surface=findViewById<SurfaceView>(R.id.videoSurface)
        surface.holder.addCallback(object:SurfaceHolder.Callback {
            override fun surfaceCreated(holder:SurfaceHolder) {
                videoActive=true
                videoWorker.execute {if(NATIVE_READY)nativeSetVideoSurface(holder.surface)}
                videoHandler.removeCallbacks(videoTick)
                videoHandler.post(videoTick)
            }
            override fun surfaceChanged(holder:SurfaceHolder,format:Int,width:Int,height:Int) {
                // EGL viewport is updated on the dedicated video worker.
            }
            override fun surfaceDestroyed(holder:SurfaceHolder) {
                videoActive=false
                videoHandler.removeCallbacks(videoTick)
                videoWorker.execute {if(NATIVE_READY)nativeSetVideoSurface(null)}
            }
        })
    }
    private fun touchButton(id:Int,mask:Int){
        findViewById<Button>(id).setOnTouchListener {_,event ->
            when(event.actionMasked){
                MotionEvent.ACTION_DOWN->{if(RUST_READY)nativeSetButton(mask,true);true}
                MotionEvent.ACTION_UP,MotionEvent.ACTION_CANCEL->{if(RUST_READY)nativeSetButton(mask,false);true}
                else->true
            }
        }
    }
    private fun setupPad(){
        val left=findViewById<AnalogPad>(R.id.leftStick)
        val right=findViewById<AnalogPad>(R.id.rightStick)
        left.onAxis={x,y->if(RUST_READY){nativeSetAxis(0,x);nativeSetAxis(1,y)}}
        right.onAxis={x,y->if(RUST_READY){nativeSetAxis(2,x);nativeSetAxis(3,y)}}
        touchButton(R.id.padLB,0x0100)
        touchButton(R.id.padRB,0x0200)
        touchButton(R.id.padStart,0x0010)
        touchButton(R.id.padBack,0x0020)
        touchButton(R.id.padX,0x4000)
        touchButton(R.id.padY,0x8000)
        touchButton(R.id.padUp,0x0001)
        touchButton(R.id.padDown,0x0002)
        touchButton(R.id.padA,0x1000)
        touchButton(R.id.padB,0x2000)
        touchButton(R.id.padLeft,0x0004)
        touchButton(R.id.padRight,0x0008)
    }
    override fun onKeyDown(keyCode:Int,event:KeyEvent?):Boolean {
        val mask=when(keyCode){
            KeyEvent.KEYCODE_BUTTON_A->0x1000
            KeyEvent.KEYCODE_BUTTON_B->0x2000
            KeyEvent.KEYCODE_DPAD_LEFT->0x0004
            KeyEvent.KEYCODE_DPAD_RIGHT->0x0008
            else->0
        }
        if(mask!=0 && RUST_READY){nativeSetButton(mask,true);return true}
        return super.onKeyDown(keyCode,event)
    }
    override fun onKeyUp(keyCode:Int,event:KeyEvent?):Boolean {
        val mask=when(keyCode){
            KeyEvent.KEYCODE_BUTTON_A->0x1000
            KeyEvent.KEYCODE_BUTTON_B->0x2000
            KeyEvent.KEYCODE_DPAD_LEFT->0x0004
            KeyEvent.KEYCODE_DPAD_RIGHT->0x0008
            else->0
        }
        if(mask!=0 && RUST_READY){nativeSetButton(mask,false);return true}
        return super.onKeyUp(keyCode,event)
    }
    override fun onGenericMotionEvent(event:MotionEvent):Boolean {
        if(event.source and android.view.InputDevice.SOURCE_JOYSTICK==android.view.InputDevice.SOURCE_JOYSTICK && RUST_READY){
            nativeSetAxis(0,(event.getAxisValue(MotionEvent.AXIS_X)*32767f).toInt())
            nativeSetAxis(1,(event.getAxisValue(MotionEvent.AXIS_Y)*32767f).toInt())
            return true
        }
        return super.onGenericMotionEvent(event)
    }
    override fun onDestroy(){
        if(gameOverlay!=null)exitGameMode(false)
        videoActive=false
        videoHandler.removeCallbacks(videoTick)
        continueRunning.set(false)
        if(RUST_READY)nativeStopSession()
        worker.shutdownNow()
        videoWorker.execute {if(NATIVE_READY)nativeSetVideoSurface(null)}
        videoWorker.shutdown()
        if(NATIVE_READY)nativeClearXexKey()
        super.onDestroy()
    }
}