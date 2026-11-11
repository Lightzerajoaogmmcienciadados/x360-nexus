package com.x360nexus;

import android.app.Activity;
import android.os.Bundle;
import android.content.Intent;
import android.database.Cursor;
import android.graphics.Bitmap;
import android.graphics.Color;
import android.net.Uri;
import android.provider.OpenableColumns;
import android.view.View;
import android.widget.*;
import java.io.InputStream;
import java.util.Locale;

public class MainActivity extends Activity {

    private static final int PICK_FILE = 100;
    private static final int GPU_WIDTH = 640;
    private static final int GPU_HEIGHT = 360;

    private static final boolean RUST_AVAILABLE;

    static {
        boolean loaded;
        try {
            System.loadLibrary("x360nexus");
            loaded = true;
        } catch (UnsatisfiedLinkError error) {
            loaded = false;
        }
        RUST_AVAILABLE = loaded;
    }

    private native String nativeSelfTest();
    private native int[] nativeGpuFrame();

    private TextView result;
    private ImageView gpuScreen;
    private LinearLayout root;

    private final int green =
        Color.rgb(30, 210, 95);

    private TextView text(
        String s,
        int size,
        int color
    ) {
        TextView v = new TextView(this);
        v.setText(s);
        v.setTextSize(size);
        v.setTextColor(color);
        v.setPadding(10, 15, 10, 15);
        return v;
    }

    private Button button(String title) {
        Button b = new Button(this);
        b.setText(title);
        b.setAllCaps(false);
        return b;
    }

    @Override
    public void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);

        ScrollView scroll = new ScrollView(this);

        root = new LinearLayout(this);
        root.setOrientation(LinearLayout.VERTICAL);
        root.setPadding(25, 35, 25, 30);
        root.setBackgroundColor(
            Color.rgb(10, 17, 13)
        );

        root.addView(text(
            "X360 NEXUS",
            32,
            green
        ));

        root.addView(text(
            "Android 0.6 - PROJECT XENON\n" +
            "CPU PowerPC | GPU Rust | Kernel",
            16,
            Color.WHITE
        ));

        // File selection
        Button open = button(
            "SELECIONAR ARQUIVO DE JOGO"
        );

        open.setOnClickListener(v -> {
            Intent i = new Intent(
                Intent.ACTION_OPEN_DOCUMENT
            );

            i.addCategory(
                Intent.CATEGORY_OPENABLE
            );

            i.setType("*/*");

            startActivityForResult(
                i,
                PICK_FILE
            );
        });

        root.addView(open);

        // Rust CPU and kernel tests
        Button test = button(
            "TESTAR CPU RUST + KERNEL"
        );

        test.setOnClickListener(v -> {
            if (!RUST_AVAILABLE) {
                result.setText(
                    "Biblioteca Rust nao encontrada."
                );
                return;
            }

            try {
                result.setText(nativeSelfTest());
            } catch (Throwable error) {
                result.setText(
                    "Erro: " + error.getMessage()
                );
            }
        });

        root.addView(test);

        // GPU visual test button
        Button gpuButton = button(
            "RENDERIZAR GPU XENOS"
        );

        root.addView(gpuButton);

        root.addView(text(
            "GPU EXPERIMENTAL - 640x360",
            18,
            green
        ));

        gpuScreen = new ImageView(this);

        gpuScreen.setBackgroundColor(
            Color.BLACK
        );

        gpuScreen.setScaleType(
            ImageView.ScaleType.FIT_CENTER
        );

        gpuScreen.setAdjustViewBounds(true);

        LinearLayout.LayoutParams params =
            new LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            );

        params.topMargin = 15;
        params.bottomMargin = 20;

        gpuScreen.setLayoutParams(params);

        Bitmap initial = Bitmap.createBitmap(
            GPU_WIDTH,
            GPU_HEIGHT,
            Bitmap.Config.ARGB_8888
        );

        initial.eraseColor(Color.BLACK);
        gpuScreen.setImageBitmap(initial);

        root.addView(gpuScreen);

        gpuButton.setOnClickListener(v -> {
            renderGpu();
        });

        result = text(
            "X360 Nexus v0.6 pronto.\n" +
            "CPU, memoria e GPU experimentais.\n" +
            "Selecione um teste para iniciar.\n\n" +
            "Jogos comerciais nao suportados.",
            16,
            Color.LTGRAY
        );

        root.addView(result);

        scroll.addView(root);
        setContentView(scroll);
    }

    private void renderGpu() {
        if (!RUST_AVAILABLE) {
            result.setText(
                "Biblioteca Rust nao encontrada."
            );
            return;
        }

        try {
            int[] pixels = nativeGpuFrame();

            if (pixels == null ||
                pixels.length != GPU_WIDTH * GPU_HEIGHT) {
                result.setText(
                    "GPU retornou dados invalidos."
                );
                return;
            }

            Bitmap bitmap = Bitmap.createBitmap(
                pixels,
                GPU_WIDTH,
                GPU_HEIGHT,
                Bitmap.Config.ARGB_8888
            );

            gpuScreen.setImageBitmap(bitmap);

            result.setText(
                "X360 NEXUS v0.6\n" +
                "GPU Rust: OK\n" +
                "Framebuffer: 640x360\n" +
                "Renderizacao visual: OK\n" +
                "Pixels: " + pixels.length + "\n" +
                "Backend: software experimental\n" +
                "Xenos real: nao implementado"
            );

        } catch (Throwable error) {
            result.setText(
                "Falha na GPU: " +
                error.getMessage()
            );
        }
    }

    @Override
    protected void onActivityResult(
        int request,
        int status,
        Intent data
    ) {
        super.onActivityResult(
            request,
            status,
            data
        );

        if (request != PICK_FILE ||
            status != RESULT_OK ||
            data == null ||
            data.getData() == null) {
            return;
        }

        Uri uri = data.getData();
        String name = uri.getLastPathSegment();
        long size = -1;

        try (Cursor c = getContentResolver()
                .query(uri, null, null, null, null)) {

            if (c != null && c.moveToFirst()) {
                int n = c.getColumnIndex(
                    OpenableColumns.DISPLAY_NAME
                );

                int z = c.getColumnIndex(
                    OpenableColumns.SIZE
                );

                if (n >= 0) {
                    name = c.getString(n);
                }

                if (z >= 0 && !c.isNull(z)) {
                    size = c.getLong(z);
                }
            }

        } catch (Exception ignored) {}

        String magic = "nao identificado";

        try (InputStream in = getContentResolver()
                .openInputStream(uri)) {

            if (in != null) {
                byte[] bytes = new byte[16];
                int count = in.read(bytes);

                if (count >= 4) {
                    magic = String.format(
                        Locale.US,
                        "%02X %02X %02X %02X",
                        bytes[0] & 255,
                        bytes[1] & 255,
                        bytes[2] & 255,
                        bytes[3] & 255
                    );

                    if (bytes[0] == 'X' &&
                        bytes[1] == 'E' &&
                        bytes[2] == 'X' &&
                        bytes[3] == '2') {
                        magic += " (XEX2)";
                    }
                }
            }

        } catch (Exception e) {
            magic = "Erro ao ler arquivo";
        }

        result.setText(
            "Arquivo: " + name +
            "\nTamanho: " +
            (size < 0 ?
                "desconhecido" :
                size + " bytes") +
            "\nPrimeiros bytes: " + magic +
            "\n\nAnalise informativa apenas." +
            "\nExecucao de jogos nao implementada."
        );
    }
}
