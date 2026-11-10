package com.x360nexus;

import android.app.Activity;
import android.os.Bundle;
import android.content.Intent;
import android.database.Cursor;
import android.graphics.Color;
import android.net.Uri;
import android.provider.OpenableColumns;
import android.view.View;
import android.widget.*;
import java.io.InputStream;
import java.util.Locale;

public class MainActivity extends Activity {
    private static final int PICK_FILE = 100;
    private static final boolean RUST_AVAILABLE;
    static {
        boolean loaded;
        try { System.loadLibrary("x360_nexus_core"); loaded = true; }
        catch (UnsatisfiedLinkError error) { loaded = false; }
        RUST_AVAILABLE = loaded;
    }
    private native String nativeSelfTest();
    private TextView result;
    private LinearLayout root;
    private int green = Color.rgb(30, 210, 95);

    private TextView text(String s, int size, int color) {
        TextView v = new TextView(this);
        v.setText(s); v.setTextSize(size); v.setTextColor(color);
        v.setPadding(10, 15, 10, 15);
        return v;
    }

    @Override public void onCreate(Bundle b) {
        super.onCreate(b);
        ScrollView scroll = new ScrollView(this);
        root = new LinearLayout(this);
        root.setOrientation(LinearLayout.VERTICAL);
        root.setPadding(25, 40, 25, 25);
        root.setBackgroundColor(Color.rgb(10, 17, 13));
        root.addView(text("X360 NEXUS", 32, green));
        root.addView(text("Android 0.5 • Nucleo Rust JNI e analisador experimental de arquivos Xbox 360", 15, Color.WHITE));
        Button open = new Button(this);
        open.setText("SELECIONAR ARQUIVO DE JOGO");
        open.setOnClickListener(v -> {
            Intent i = new Intent(Intent.ACTION_OPEN_DOCUMENT);
            i.addCategory(Intent.CATEGORY_OPENABLE);
            i.setType("*/*");
            startActivityForResult(i, PICK_FILE);
        });
        root.addView(open);
        Button test = new Button(this);
        test.setText("TESTAR CPU RUST + KERNEL");
        test.setOnClickListener(v -> {
            if (!RUST_AVAILABLE) {
                result.setText("Biblioteca Rust nao encontrada no APK.");
                return;
            }
            try { result.setText(nativeSelfTest()); }
            catch (Throwable error) { result.setText("Erro no nucleo: " + error.getMessage()); }
        });
        root.addView(test);
        result = text("Selecione um arquivo ISO, XEX ou outro formato para inspecionar.\n\nA execução de jogos ainda não está implementada.", 16, Color.LTGRAY);
        root.addView(result);
        scroll.addView(root);
        setContentView(scroll);
    }

    @Override protected void onActivityResult(int request, int status, Intent data) {
        super.onActivityResult(request, status, data);
        if (request != PICK_FILE || status != RESULT_OK || data == null || data.getData() == null) return;
        Uri uri = data.getData();
        String name = uri.getLastPathSegment();
        long size = -1;
        try (Cursor c = getContentResolver().query(uri, null, null, null, null)) {
            if (c != null && c.moveToFirst()) {
                int n = c.getColumnIndex(OpenableColumns.DISPLAY_NAME);
                int z = c.getColumnIndex(OpenableColumns.SIZE);
                if (n >= 0) name = c.getString(n);
                if (z >= 0 && !c.isNull(z)) size = c.getLong(z);
            }
        } catch (Exception ignored) {}
        String magic = "não identificado";
        try (InputStream in = getContentResolver().openInputStream(uri)) {
            if (in != null) {
                byte[] bytes = new byte[16];
                int count = in.read(bytes);
                if (count >= 4) {
                    magic = String.format(Locale.US, "%02X %02X %02X %02X", bytes[0] & 255, bytes[1] & 255, bytes[2] & 255, bytes[3] & 255);
                    if (bytes[0] == 'X' && bytes[1] == 'E' && bytes[2] == 'X' && bytes[3] == '2') magic += " (assinatura XEX2)";
                }
            }
        } catch (Exception e) { magic = "erro ao ler: " + e.getMessage(); }
        result.setText("Arquivo: " + name + "\nTamanho: " + (size < 0 ? "desconhecido" : size + " bytes") + "\nPrimeiros bytes: " + magic + "\n\nEste aplicativo NÃO executa jogos de Xbox 360. A análise é apenas informativa.");
    }
}
