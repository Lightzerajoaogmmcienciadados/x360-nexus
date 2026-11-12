package com.x360nexus;

import android.app.Activity;
import android.os.Bundle;
import android.content.Intent;
import android.database.Cursor;
import android.graphics.Color;
import android.net.Uri;
import android.os.ParcelFileDescriptor;
import android.provider.OpenableColumns;
import android.widget.Button;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.TextView;

import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;

/** X360 Nexus v4.2: ISO/XDVDFS read-only inspection and XEX guest RAM staging. */
public class MainActivity extends Activity {
    private static final int PICK_ISO = 100;
    private static final int SAVE_XEX = 101;
    private static final int GREEN = Color.rgb(30, 210, 95);
    private static final boolean RUST_READY;
    private static final boolean XDVDFS_READY;

    static {
        boolean rust = false, xdvdfs = false;
        try { System.loadLibrary("x360nexus"); rust = true; }
        catch (UnsatisfiedLinkError ignored) { }
        try { System.loadLibrary("x360xdvdfs"); xdvdfs = true; }
        catch (UnsatisfiedLinkError ignored) { }
        RUST_READY = rust;
        XDVDFS_READY = xdvdfs;
    }

    private native String nativeSelfTest();
    private native String nativeInspectXdvdfs(int fd);
    private native String nativeLoadDefaultXex(int fd);
    private native String nativeSaveDefaultXex(int inputFd, int outputFd);
    private native String nativeGuestRamStatus();

    private final ExecutorService worker = Executors.newSingleThreadExecutor();
    private final java.util.ArrayList<Button> actionButtons = new java.util.ArrayList<>();
    private TextView result;
    private Uri selectedIso;
    private boolean busy = false;

    private TextView text(String value, int size, int color) {
        TextView t = new TextView(this);
        t.setText(value);
        t.setTextSize(size);
        t.setTextColor(color);
        t.setPadding(8, 12, 8, 12);
        t.setTextIsSelectable(true);
        return t;
    }
    private Button button(String title, Runnable action, LinearLayout root) {
        Button b = new Button(this);
        b.setText(title);
        b.setAllCaps(false);
        b.setOnClickListener(v -> action.run());
        root.addView(b);
        actionButtons.add(b);
        return b;
    }
    private void show(String message) { result.setText(message); }
    private void setBusy(boolean b) {
        busy = b;
        for (Button button : actionButtons) button.setEnabled(!b);
    }
    private interface IsoWork { String run(int fd); }
    private void onIso(String title, IsoWork work) {
        if (busy) return;
        if (!XDVDFS_READY) { show("Erro: biblioteca C++ XDVDFS indisponivel."); return; }
        if (selectedIso == null) { show("Selecione uma ISO primeiro."); return; }
        final Uri uri = selectedIso;
        setBusy(true);
        show(title + "\nLendo os setores da ISO selecionada...");
        worker.execute(() -> {
            String response;
            try (ParcelFileDescriptor document = getContentResolver().openFileDescriptor(uri, "r")) {
                if (document == null) throw new Exception("Documento nao pode ser aberto");
                response = work.run(document.getFd());
            } catch (Throwable t) {
                response = "Falha ao abrir ISO: " + t.getMessage();
            }
            final String value = response;
            runOnUiThread(() -> { setBusy(false); show(value); });
        });
    }

    @Override public void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        ScrollView scroll = new ScrollView(this);
        LinearLayout root = new LinearLayout(this);
        root.setOrientation(LinearLayout.VERTICAL);
        root.setPadding(24, 36, 24, 36);
        root.setBackgroundColor(Color.rgb(7, 15, 20));
        root.addView(text("PROJECT XENON", 32, GREEN));
        root.addView(text("4.2: GALAXY | X360 Nexus Android\nCPU PowerPC + Leitor XDVDFS em C++", 17, Color.WHITE));
        root.addView(text("ISO/XISO/XGD2/XGD3 | Arquivos | default.xex", 17, GREEN));

        button("1. SELECIONAR ISO", () -> {
            Intent intent = new Intent(Intent.ACTION_OPEN_DOCUMENT);
            intent.addCategory(Intent.CATEGORY_OPENABLE);
            intent.setType("*/*");
            intent.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION |
                    Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION);
            startActivityForResult(intent, PICK_ISO);
        }, root);
        button("2. ANALISAR ISO E LISTAR ARQUIVOS", () ->
                onIso("Analisando XDVDFS...", this::nativeInspectXdvdfs), root);
        button("3. CARREGAR default.xex NA RAM EMULADA", () ->
                onIso("Carregando default.xex na RAM...", this::nativeLoadDefaultXex), root);
        button("4. EXTRAIR default.xex PARA ARQUIVO", () -> {
            if (busy || selectedIso == null) { show("Selecione uma ISO primeiro."); return; }
            Intent intent = new Intent(Intent.ACTION_CREATE_DOCUMENT);
            intent.addCategory(Intent.CATEGORY_OPENABLE);
            intent.setType("application/octet-stream");
            intent.putExtra(Intent.EXTRA_TITLE, "default.xex");
            startActivityForResult(intent, SAVE_XEX);
        }, root);
        button("CONSULTAR MEMORIA RAM EMULADA", () -> {
            show(XDVDFS_READY ? nativeGuestRamStatus() : "Nucleo C++ indisponivel");
        }, root);
        button("TESTAR CPU POWERPC + KERNEL (RUST)", () -> {
            if (!RUST_READY) { show("Nucleo Rust JNI indisponivel"); return; }
            try { show(nativeSelfTest()); }
            catch (Throwable t) { show("Erro no nucleo Rust: " + t.getMessage()); }
        }, root);

        result = text("X360 Nexus v4.2\n\nRust JNI: " + (RUST_READY ? "OK" : "Indisponivel") +
                "\nXDVDFS JNI C++: " + (XDVDFS_READY ? "OK" : "Indisponivel") +
                "\n\nA leitura ISO nao executa o jogo. Copiar o default.xex para RAM emulada " +
                "nao equivale a descriptografar, carregar secoes ou executar PowerPC.",
                16, Color.LTGRAY);
        root.addView(result);
        scroll.addView(root);
        setContentView(scroll);
    }

    @Override protected void onActivityResult(int request, int status, Intent data) {
        super.onActivityResult(request, status, data);
        if (status != RESULT_OK || data == null || data.getData() == null) return;
        final Uri uri = data.getData();
        if (request == PICK_ISO) {
            selectedIso = uri;
            try {
                getContentResolver().takePersistableUriPermission(uri,
                        Intent.FLAG_GRANT_READ_URI_PERMISSION);
            } catch (SecurityException ignored) { }
            String name = uri.getLastPathSegment();
            long bytes = -1;
            try (Cursor c = getContentResolver().query(uri, null, null, null, null)) {
                if (c != null && c.moveToFirst()) {
                    int i = c.getColumnIndex(OpenableColumns.DISPLAY_NAME);
                    if (i >= 0) name = c.getString(i);
                    i = c.getColumnIndex(OpenableColumns.SIZE);
                    if (i >= 0 && !c.isNull(i)) bytes = c.getLong(i);
                }
            } catch (Exception ignored) { }
            show("ISO selecionada: " + name + "\nTamanho: " +
                    (bytes >= 0 ? bytes + " bytes" : "desconhecido") +
                    "\nToque em ANALISAR ISO para ler o descritor XDVDFS.");
        } else if (request == SAVE_XEX) {
            if (busy || selectedIso == null) { show("Selecione uma ISO primeiro."); return; }
            if (!XDVDFS_READY) { show("Nucleo XDVDFS nao carregado."); return; }
            final Uri src = selectedIso;
            setBusy(true);
            show("Extraindo default.xex para o documento escolhido...");
            worker.execute(() -> {
                String msg;
                try (ParcelFileDescriptor input = getContentResolver().openFileDescriptor(src,"r");
                     ParcelFileDescriptor output = getContentResolver().openFileDescriptor(uri,"w")) {
                    if (input == null || output == null) throw new Exception("Arquivo de origem/destino nao pode ser aberto");
                    msg = nativeSaveDefaultXex(input.getFd(),output.getFd());
                } catch (Throwable t) { msg = "Falha na extracao: " + t.getMessage(); }
                final String value = msg;
                runOnUiThread(() -> { setBusy(false); show(value); });
            });
        }
    }

    @Override protected void onDestroy() {
        worker.shutdown();
        super.onDestroy();
    }
}
