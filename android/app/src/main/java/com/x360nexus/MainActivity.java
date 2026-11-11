
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
import android.view.Gravity;
import android.view.MotionEvent;
import android.widget.*;

import java.io.InputStream;
import java.util.Locale;

public class MainActivity extends Activity {

    private static final int PICK_FILE = 100;
    private static final int WIDTH = 640;
    private static final int HEIGHT = 360;
    private static final int SIZE = 48;

    private static final int MODE_2D = 0;
    private static final int MODE_3D = 1;
    private static final int MODE_ADVANCED = 2;

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
    private native int[] nativeGpu3DFrame(
        float yaw, float pitch
    );
    private native int[] nativeGpuAdvancedFrame(
        float yaw, float pitch
    );

    private TextView result;
    private TextView fpsText;
    private ImageView screen;
    private Bitmap bitmap;

    private final Handler handler =
        new Handler(Looper.getMainLooper());

    private boolean running = false;
    private int mode = MODE_2D;

    private int playerX = 296;
    private int playerY = 156;
    private int dx = 0;
    private int dy = 0;

    private float cameraYaw = 0.0f;
    private float cameraPitch = 0.3f;

    private long tick = 0;
    private long lastFrame = 0;
    private long fpsStart = 0;
    private int fpsFrames = 0;
    private int fps = 0;

    private final int green =
        Color.rgb(30, 210, 95);

    private TextView label(
        String value, int size, int color
    ) {
        TextView text = new TextView(this);
        text.setText(value);
        text.setTextSize(size);
        text.setTextColor(color);
        text.setPadding(8, 12, 8, 12);
        return text;
    }

    private Button button(String title) {
        Button b = new Button(this);
        b.setText(title);
        b.setAllCaps(false);
        return b;
    }

    private String modeName() {
        switch (mode) {
            case MODE_3D:
                return "GPU 3D";
            case MODE_ADVANCED:
                return "GPU 3D ADVANCED";
            default:
                return "GPU 2D";
        }
    }

    private final Runnable frameLoop = new Runnable() {
        @Override
        public void run() {
            if (!running) return;

            long now = SystemClock.uptimeMillis();

            if (now - lastFrame >= 33) {
                lastFrame = now;

                if (mode != MODE_2D) {
                    cameraYaw += 0.012f;
                    cameraYaw += dx * 0.025f;
                    cameraPitch += dy * 0.025f;

                    cameraPitch = Math.max(
                        -1.2f,
                        Math.min(1.2f, cameraPitch)
                    );
                } else {
                    playerX = Math.max(
                        0,
                        Math.min(
                            WIDTH - SIZE,
                            playerX + dx * 5
                        )
                    );
                    playerY = Math.max(
                        0,
                        Math.min(
                            HEIGHT - SIZE,
                            playerY + dy * 5
                        )
                    );
                }

                tick++;
                renderFrame();

                if (!running) return;

                fpsFrames++;

                if (now - fpsStart >= 1000) {
                    long elapsed = now - fpsStart;
                    fps = (int) (
                        fpsFrames * 1000L / elapsed
                    );
                    fpsFrames = 0;
                    fpsStart = now;
                    updateFps();
                }
            }

            if (running) {
                handler.postDelayed(this, 5);
            }
        }
    };

    private void updateFps() {
        if (mode != MODE_2D) {
            fpsText.setText(
                modeName() +
                " | FPS: " + fps +
                " | Frame: " + tick +
                "\nYaw: " +
                String.format(
                    Locale.US, "%.2f", cameraYaw
                ) +
                " | Pitch: " +
                String.format(
                    Locale.US, "%.2f", cameraPitch
                )
            );
        } else {
            fpsText.setText(
                "GPU 2D | FPS: " + fps +
                " | Frame: " + tick +
                " | Posicao: " +
                playerX + "," + playerY
            );
        }
    }

    private void renderFrame() {
        if (!RUST_AVAILABLE) {
            stopAnimation();
            result.setText(
                "Biblioteca Rust indisponivel."
            );
            return;
        }

        try {
            int[] pixels;

            switch (mode) {
                case MODE_3D:
                    pixels = nativeGpu3DFrame(
                        cameraYaw, cameraPitch
                    );
                    break;

                case MODE_ADVANCED:
                    pixels = nativeGpuAdvancedFrame(
                        cameraYaw, cameraPitch
                    );
                    break;

                default:
                    pixels = nativeGpuFrameDynamic(
                        playerX, playerY, tick
                    );
                    break;
            }

            if (pixels == null ||
                pixels.length != WIDTH * HEIGHT) {
                stopAnimation();
                result.setText(
                    "Erro: framebuffer invalido."
                );
                return;
            }

            bitmap.setPixels(
                pixels,
                0,
                WIDTH,
                0,
                0,
                WIDTH,
                HEIGHT
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
            result.setText(
                "Biblioteca Rust nao encontrada."
            );
            return;
        }

        running = true;
        fpsFrames = 0;
        fps = 0;
        fpsStart = SystemClock.uptimeMillis();
        lastFrame = 0;

        handler.post(frameLoop);

        String details;

        switch (mode) {
            case MODE_3D:
                details =
                    "GPU 3D experimental ativa.\n" +
                    "Cubo giratorio e Z-buffer.\n" +
                    "Setas controlam a camera.\n";
                break;

            case MODE_ADVANCED:
                details =
                    "GPU 3D avancada ativa.\n" +
                    "Multiplos cubos 3D.\n" +
                    "Texturas procedurais.\n" +
                    "Iluminacao e Z-buffer.\n" +
                    "Setas controlam a rotacao.\n";
                break;

            default:
                details =
                    "GPU 2D experimental ativa.\n" +
                    "Setas movem o quadrado.\n";
                break;
        }

        result.setText(
            "X360 NEXUS v0.8\n" +
            details +
            "Backend: Rust software.\n" +
            "Xenos real: nao implementado."
        );
    }

    private void stopAnimation() {
        running = false;
        dx = 0;
        dy = 0;
        handler.removeCallbacks(frameLoop);
    }

    private void switchMode(int newMode) {
        stopAnimation();

        mode = newMode;
        tick = 0;
        cameraYaw = 0.0f;
        cameraPitch = 0.3f;

        startAnimation();
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
            "Android 0.8 - PROJECT XENON ADVANCED\n" +
            "PowerPC | GPU 2D/3D | FPS | Controles",
            16, Color.WHITE
        ));

        Button open = button(
            "SELECIONAR ARQUIVO DE JOGO"
        );
        root.addView(open);

        Button test = button(
            "TESTAR CPU RUST + KERNEL"
        );
        root.addView(test);

        Button start2D = button(
            "INICIAR GPU 2D"
        );
        root.addView(start2D);

        Button start3D = button(
            "INICIAR GPU 3D - CUBO"
        );
        root.addView(start3D);

        Button advanced = button(
            "INICIAR GPU 3D ADVANCED"
        );
        root.addView(advanced);

        Button pause = button(
            "PAUSAR GPU"
        );
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

        root.addView(
            screen,
            new LinearLayout.LayoutParams(-1, -2)
        );

        fpsText = label(
            "GPU parada | FPS: 0",
            16, green
        );
        root.addView(fpsText);

        root.addView(label(
            "CONTROLES VIRTUAIS",
            18, green
        ));

        LinearLayout top = new LinearLayout(this);
        top.setGravity(Gravity.CENTER);
        top.addView(direction("▲", 0, -1));
        root.addView(top);

        LinearLayout middle = new LinearLayout(this);
        middle.setGravity(Gravity.CENTER);
        middle.addView(direction("◀", -1, 0));
        middle.addView(direction("▼", 0, 1));
        middle.addView(direction("▶", 1, 0));
        root.addView(middle);

        result = label(
            "X360 Nexus v0.8 pronto.\n" +
            "Selecione GPU 2D, 3D ou Advanced.\n" +
            "Jogos comerciais nao suportados.",
            16, Color.LTGRAY
        );
        root.addView(result);

        open.setOnClickListener(v -> {
            Intent intent = new Intent(
                Intent.ACTION_OPEN_DOCUMENT
            );
            intent.addCategory(
                Intent.CATEGORY_OPENABLE
            );
            intent.setType("*/*");

            startActivityForResult(
                intent, PICK_FILE
            );
        });

        test.setOnClickListener(v -> {
            if (!RUST_AVAILABLE) {
                result.setText(
                    "Biblioteca Rust indisponivel."
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

        start2D.setOnClickListener(
            v -> switchMode(MODE_2D)
        );

        start3D.setOnClickListener(
            v -> switchMode(MODE_3D)
        );

        advanced.setOnClickListener(
            v -> switchMode(MODE_ADVANCED)
        );

        pause.setOnClickListener(v -> {
            stopAnimation();
            fpsText.setText(
                "GPU pausada | FPS: 0"
            );
            result.setText("GPU pausada.");
        });

        scroll.addView(root);
        setContentView(scroll);
    }

    @Override
    protected void onPause() {
        stopAnimation();
        super.onPause();
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
        super.onActivityResult(
            request, status, data
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

        try (Cursor cursor =
                getContentResolver().query(
                    uri, null, null, null, null
                )) {

            if (cursor != null &&
                cursor.moveToFirst()) {

                int nameColumn =
                    cursor.getColumnIndex(
                        OpenableColumns.DISPLAY_NAME
                    );

                int sizeColumn =
                    cursor.getColumnIndex(
                        OpenableColumns.SIZE
                    );

                if (nameColumn >= 0) {
                    name = cursor.getString(
                        nameColumn
                    );
                }

                if (sizeColumn >= 0 &&
                    !cursor.isNull(sizeColumn)) {
                    size = cursor.getLong(
                        sizeColumn
                    );
                }
            }
        } catch (Exception ignored) {}

        String magic = "nao identificado";

        try (InputStream in =
                getContentResolver()
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
            "\nInspecao experimental." +
            "\nExecucao nao implementada."
        );
    }
}
