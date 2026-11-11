
package com.x360nexus;

import android.app.Activity;
import android.os.Bundle;
import android.content.Intent;
import android.database.Cursor;
import android.graphics.Color;
import android.net.Uri;
import android.provider.OpenableColumns;
import android.widget.*;
import java.io.InputStream;
import java.util.Locale;

public class MainActivity extends Activity {

    private static final int PICK_FILE = 100;
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

    private TextView result;
    private LinearLayout root;
    private final int green = Color.rgb(30, 210, 95);

    private TextView text(
        String value, int size, int color
    ) {
        TextView view = new TextView(this);
        view.setText(value);
        view.setTextSize(size);
        view.setTextColor(color);
        view.setPadding(10, 15, 10, 15);
        return view;
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
        root.setPadding(25, 40, 25, 25);
        root.setBackgroundColor(Color.rgb(10, 17, 13));

        root.addView(text(
            "X360 NEXUS",
            32,
            green
        ));

        root.addView(text(
            "Android 4.0 - PROJECT XENON\n" +
            "PowerPC | GPU Experimental | Kernel\n" +
            "Emulador experimental Rust JNI",
            15,
            Color.WHITE
        ));

        root.addView(text(
            "X360 NEXUS v4.0\n" +
            "PROJECT XENON\n" +
            "Nucleo experimental PowerPC",
            18,
            green
        ));

        Button open = button(
            "SELECIONAR ARQUIVO DE JOGO"
        );

        open.setOnClickListener(v -> {
            Intent intent = new Intent(
                Intent.ACTION_OPEN_DOCUMENT
            );

            intent.addCategory(
                Intent.CATEGORY_OPENABLE
            );

            intent.setType("*/*");

            startActivityForResult(
                intent,
                PICK_FILE
            );
        });

        root.addView(open);

        Button test = button(
            "TESTAR CPU POWERPC + KERNEL"
        );

        root.addView(test);

        result = text(
            "X360 Nexus v4.0 iniciado.\n\n" +
            "Nucleo Rust JNI experimental.\n" +
            "CPU PowerPC: testes disponiveis.\n" +
            "Memoria: experimental.\n" +
            "GPU Xenos: nao implementada.\n" +
            "Execucao de jogos: indisponivel.",
            16,
            Color.LTGRAY
        );

        root.addView(result);

        test.setOnClickListener(v -> {
            if (!RUST_AVAILABLE) {
                result.setText(
                    "Erro: biblioteca Rust nao encontrada."
                );
                return;
            }

            try {
                result.setText(nativeSelfTest());
            } catch (Throwable error) {
                result.setText(
                    "Erro no nucleo Rust:\n" +
                    error.getMessage()
                );
            }
        });

        scroll.addView(root);
        setContentView(scroll);
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

        try (Cursor cursor =
                getContentResolver().query(
                    uri, null, null, null, null
                )) {

            if (cursor != null &&
                cursor.moveToFirst()) {

                int nameIndex =
                    cursor.getColumnIndex(
                        OpenableColumns.DISPLAY_NAME
                    );

                int sizeIndex =
                    cursor.getColumnIndex(
                        OpenableColumns.SIZE
                    );

                if (nameIndex >= 0) {
                    name = cursor.getString(
                        nameIndex
                    );
                }

                if (sizeIndex >= 0 &&
                    !cursor.isNull(sizeIndex)) {
                    size = cursor.getLong(
                        sizeIndex
                    );
                }
            }

        } catch (Exception ignored) {}

        String magic = "Nao identificado";

        try (InputStream in =
                getContentResolver()
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

        } catch (Exception error) {
            magic = "Erro de leitura";
        }

        result.setText(
            "X360 NEXUS v4.0\n\n" +
            "Arquivo: " + name +
            "\nTamanho: " +
            (size < 0
                ? "Desconhecido"
                : size + " bytes") +
            "\nAssinatura: " + magic +
            "\n\nArquivo selecionado para inspecao." +
            "\nExecucao de jogos ainda nao implementada."
        );
    }
}
