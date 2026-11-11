
package com.x360nexus;

import android.app.Activity;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.os.SystemClock;
import android.content.Intent;
import android.database.Cursor;
import android.graphics.Bitmap;
import android.graphics.Color;
import android.net.Uri;
import android.provider.OpenableColumns;
import android.view.MotionEvent;
import android.widget.*;
import java.io.InputStream;
import java.util.Locale;

public class MainActivity extends Activity {

    private static final int PICK_FILE = 100;
    private static final int WIDTH = 640;
    private static final int HEIGHT = 360;
    private static final int SIZE = 48;

    private static final boolean RUST_AVAILABLE;

    static {
        boolean loaded;
        try {
            System.loadLibrary("x360nexus");
            loaded = true;
        } catch (UnsatisfiedLinkError e) {
            loaded = false;
        }
        RUST_AVAILABLE = loaded;
    }

    private native String nativeSelfTest();
    private native int[] nativeGpuFrame();
    private native int[] nativeGpuFrameDynamic(
        int x, int y, long tick
    );

    private TextView result;
    private TextView fpsText;
    private ImageView screen;
    private Bitmap bitmap;

    private final Handler handler =
        new Handler(Looper.getMainLooper());

    private boolean running = false;
    private int playerX = 296;
    private int playerY = 156;
    private int dx = 0;
    private int dy = 0;
    private long tick = 0;
    private long lastFrame = 0;
    private long fpsStart = 0;
    private int fpsFrames = 0;
    private int fps = 0;

    private final int green =
        Color.rgb(30, 210, 95);

    private TextView label(String s, int size, int color) {
        TextView t = new TextView(this);
        t.setText(s);
        t.setTextSize(size);
        t.setTextColor(color);
        t.setPadding(8, 12, 8, 12);
        return t;
    }

    private Button button(String title) {
        Button b = new Button(this);
        b.setText(title);
        b.setAllCaps(false);
        return b;
    }

    private final Runnable frameLoop = new Runnable() {
        @Override
        public void run() {
            if (!running) return;

            long now = SystemClock.uptimeMillis();

            // Target approximately 30 FPS.
            if (now - lastFrame >= 33) {
                lastFrame = now;

                playerX = Math.max(
                    0,
                    Math.min(WIDTH - SIZE, playerX + dx * 5)
                );

                playerY = Math.max(
                    0,
                    Math.min(HEIGHT - SIZE, playerY + dy * 5)
                );

                tick++;
                renderDynamic();

                fpsFrames++;

                if (now - fpsStart >= 1000) {
                    long elapsed = now - fpsStart;
                    fps = (int) (
                        fpsFrames * 1000L / elapsed
                    );
                    fpsFrames = 0;
                    fpsStart = now;

                    fpsText.setText(
                        "FPS: " + fps +
                        " | Frame: " + tick +
                        " | Posicao: " +
                        playerX + "," + playerY
                    );
                }
            }

            if (running) {
                handler.postDelayed(this, 5);
            }
        }
    };

    private void renderDynamic() {
        if (!RUST_AVAILABLE) {
            stopAnimation();
            result.setText("Biblioteca Rust indisponivel.");
            return;
        }

        try {
            int[] pixels = nativeGpuFrameDynamic(
                playerX, playerY, tick
            );

            if (pixels == null ||
                pixels.length != WIDTH * HEIGHT) {
                stopAnimation();
                result.setText(
                    "Framebuffer invalido."
                );
                return;
            }

            bitmap.setPixels(
                pixels, 0, WIDTH,
                0, 0, WIDTH, HEIGHT
            );

            screen.invalidate();

        } catch (Throwable e) {
            stopAnimation();
            result.setText(
                "Erro GPU: " + e.getMessage()
            );
        }
    }

    private void startAnimation() {
        if (running) return;

        if (!RUST_AVAILABLE) {
            result.setText("Rust JNI indisponivel.");
            return;
        }

        running = true;
        fpsFrames = 0;
        fpsStart = SystemClock.uptimeMillis();
        lastFrame = 0;

        handler.post(frameLoop);

        result.setText(
            "GPU dinamica ativa.\n" +
            "Use os controles para mover o quadrado.\n" +
            "Backend: Rust software experimental.\n" +
            "Xenos real: nao implementado."
        );
    }

    private void stopAnimation() {
        running = false;
        dx = 0;
        dy = 0;
        handler.removeCallbacks(frameLoop);
    }

    private Button direction(
        String title,
        int horizontal,
        int vertical
    ) {
        Button b = button(title);

        b.setOnTouchListener((v, event) -> {
            switch (event.getActionMasked()) {
                case MotionEvent.ACTION_DOWN:
                    dx = horizontal;
                    dy = vertical;
                    return true;

                case MotionEvent.ACTION_UP:
                case MotionEvent.ACTION_CANCEL:
                    dx = 0;
                    dy = 0;
                    return true;
            }
            return true;
        });

        return b;
    }

    @Override
    public void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);

        ScrollView scroll = new ScrollView(this);
        LinearLayout root = new LinearLayout(this);

        root.setOrientation(LinearLayout.VERTICAL);
        root.setPadding(24, 28, 24, 30);
        root.setBackgroundColor(
            Color.rgb(10, 17, 13)
        );

        root.addView(label(
            "X360 NEXUS", 32, green
        ));

        root.addView(label(
            "Android 0.6.1 - PROJECT XENON\n" +
            "PowerPC | GPU 2D | FPS | Controles",
            16, Color.WHITE
        ));

        Button open = button(
            "SELECIONAR ARQUIVO DE JOGO"
        );

        open.setOnClickListener(v -> {
            Intent i = new Intent(
                Intent.ACTION_OPEN_DOCUMENT
            );
            i.addCategory(Intent.CATEGORY_OPENABLE);
            i.setType("*/*");
            startActivityForResult(i, PICK_FILE);
        });

        root.addView(open);

        Button test = button(
            "TESTAR CPU RUST + KERNEL"
        );

        root.addView(test);

        Button start = button(
            "INICIAR GPU DINAMICA"
        );

        Button pause = button(
            "PAUSAR GPU"
        );

        root.addView(start);
        root.addView(pause);

        root.addView(label(
            "GPU EXPERIMENTAL - 640x360",
            18, green
        ));

        bitmap = Bitmap.createBitmap(
            WIDTH, HEIGHT,
            Bitmap.Config.ARGB_8888
        );

        bitmap.eraseColor(Color.BLACK);

        screen = new ImageView(this);
        screen.setImageBitmap(bitmap);
        screen.setAdjustViewBounds(true);
        screen.setScaleType(
            ImageView.ScaleType.FIT_CENTER
        );
        screen.setBackgroundColor(Color.BLACK);

        root.addView(screen, new LinearLayout.LayoutParams(
            -1, -2
        ));

        fpsText = label(
            "FPS: 0 | Frame: 0",
            16, green
        );
        root.addView(fpsText);

        root.addView(label(
            "CONTROLES VIRTUAIS",
            18, green
        ));

        LinearLayout top = new LinearLayout(this);
        top.setGravity(android.view.Gravity.CENTER);

        top.addView(direction("▲", 0, -1));
        root.addView(top);

        LinearLayout middle = new LinearLayout(this);
        middle.setGravity(android.view.Gravity.CENTER);

        middle.addView(direction("◀", -1, 0));
        middle.addView(direction("▼", 0, 1));
        middle.addView(direction("▶", 1, 0));

        root.addView(middle);

        result = label(
            "X360 Nexus v0.6.1 pronto.\n" +
            "Toque em INICIAR GPU DINAMICA.\n" +
            "Jogos comerciais nao suportados.",
            16, Color.LTGRAY
        );

        root.addView(result);

        test.setOnClickListener(v -> {
            if (!RUST_AVAILABLE) {
                result.setText(
                    "Biblioteca Rust nao encontrada."
                );
                return;
            }

            try {
                result.setText(nativeSelfTest());
            } catch (Throwable e) {
                result.setText(
                    "Erro: " + e.getMessage()
                );
            }
        });

        start.setOnClickListener(v -> startAnimation());

        pause.setOnClickListener(v -> {
            stopAnimation();
            result.setText("GPU pausada.");
        });

        scroll.addView(root);
        setContentView(scroll);
    }

    @Override
    protected void onPause() {
        super.onPause();
        stopAnimation();
    }

    @Override
    protected void onDestroy() {
        stopAnimation();
        super.onDestroy();
    }

    @Override
    protected void onActivityResult(
        int request,
        int status,
        Intent data
    ) {
        super.onActivityResult(request, status, data);

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

                if (n >= 0) name = c.getString(n);
                if (z >= 0 && !c.isNull(z)) {
                    size = c.getLong(z);
                }
            }
        } catch (Exception ignored) {}

        String magic = "nao identificado";

        try (InputStream in = getContentResolver()
                .openInputStream(uri)) {

            if (in != null) {
                byte[] bytes = new byte[4];
                int count = in.read(bytes);

                if (count == 4) {
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
            magic = "erro de leitura";
        }

        result.setText(
            "Arquivo: " + name +
            "\nTamanho: " + size + " bytes" +
            "\nAssinatura: " + magic +
            "\nAnalise informativa apenas." +
            "\nExecucao nao implementada."
        );
    }
}
