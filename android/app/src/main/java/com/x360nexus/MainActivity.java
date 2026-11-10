
package com.x360nexus;

import android.app.Activity;
import android.os.Bundle;
import android.content.Intent;
import android.content.SharedPreferences;
import android.net.Uri;
import android.graphics.Color;
import android.graphics.Typeface;
import android.view.*;
import android.widget.*;
import org.json.JSONArray;
import org.json.JSONObject;

public class MainActivity extends Activity {

    LinearLayout root, library;
    SharedPreferences prefs;
    final int GREEN = Color.rgb(16, 210, 95);
    final int BG = Color.rgb(9, 15, 12);
    final int CARD = Color.rgb(24, 35, 29);
    final int PICK_GAME = 100;
    final int PICK_COVER = 101;

    JSONArray games = new JSONArray();
    int coverIndex = -1;

    int dp(int value) {
        return (int)(value *
            getResources().getDisplayMetrics().density);
    }

    @Override
    public void onCreate(Bundle state) {
        super.onCreate(state);

        prefs = getSharedPreferences(
            "x360_library", MODE_PRIVATE);

        try {
            games = new JSONArray(
                prefs.getString("games", "[]"));
        } catch (Exception e) {
            games = new JSONArray();
        }

        render();
    }

    TextView text(String value, int size, int color) {
        TextView t = new TextView(this);
        t.setText(value);
        t.setTextSize(size);
        t.setTextColor(color);
        t.setPadding(dp(8), dp(10), dp(8), dp(10));
        return t;
    }

    Button button(String label) {
        Button b = new Button(this);
        b.setText(label);
        b.setTextColor(Color.WHITE);
        b.setBackgroundTintList(
            android.content.res.ColorStateList.valueOf(GREEN));
        return b;
    }

    void render() {
        ScrollView scroll = new ScrollView(this);
        root = new LinearLayout(this);
        root.setOrientation(LinearLayout.VERTICAL);
        root.setPadding(dp(16), dp(20), dp(16), dp(24));
        root.setBackgroundColor(BG);

        TextView title = text("X360 NEXUS", 32, GREEN);
        title.setTypeface(null, Typeface.BOLD);
        root.addView(title);

        root.addView(text(
            "GAMING HUB  •  VERSION 0.2", 13,
            Color.LTGRAY));

        Button importGame = button("+ IMPORTAR JOGO");
        root.addView(importGame);

        importGame.setOnClickListener(v ->
            openPicker(PICK_GAME, "*/*"));

        root.addView(text(
            "MINHA BIBLIOTECA", 23, Color.WHITE));

        root.addView(text(
            games.length() + " arquivo(s) salvo(s)",
            13, Color.LTGRAY));

        library = new LinearLayout(this);
        library.setOrientation(LinearLayout.VERTICAL);
        root.addView(library);

        for (int i = 0; i < games.length(); i++) {
            addGameCard(i);
        }

        if (games.length() == 0) {
            library.addView(text(
                "Nenhum jogo importado.\n" +
                "Toque em IMPORTAR JOGO para começar.",
                15, Color.LTGRAY));
        }

        root.addView(text(
            "CONTROLE", 22, Color.WHITE));

        root.addView(text(
            "Conecte um controle Bluetooth nas " +
            "configurações do Android e pressione " +
            "um botão para testar.",
            14, Color.LTGRAY));

        controllerStatus = text(
            "Aguardando entrada do controle...",
            14, GREEN);
        root.addView(controllerStatus);

        root.addView(text(
            "NÚCLEO RUST", 22, Color.WHITE));

        String rustStatus;
        try {
            rustStatus = nativeVersion();
        } catch (UnsatisfiedLinkError e) {
            rustStatus = "Núcleo nativo não instalado";
        }

        root.addView(text(rustStatus, 14, GREEN));

        root.addView(text(
            "Gerenciador experimental. " +
            "Emulação Xbox 360 ainda indisponível.",
            12, Color.LTGRAY));

        scroll.addView(root);
        setContentView(scroll);
    }

    TextView controllerStatus;

    void addGameCard(int index) {
        JSONObject game = games.optJSONObject(index);
        if (game == null) return;

        LinearLayout card = new LinearLayout(this);
        card.setOrientation(LinearLayout.VERTICAL);
        card.setPadding(dp(12), dp(12), dp(12), dp(12));
        card.setBackgroundColor(CARD);

        String name = game.optString("name", "Arquivo");
        String cover = game.optString("cover", "");

        if (!cover.isEmpty()) {
            try {
                ImageView image = new ImageView(this);
                image.setImageURI(Uri.parse(cover));
                image.setAdjustViewBounds(true);
                image.setMaxHeight(dp(180));
                card.addView(image);
            } catch (Exception ignored) {}
        }

        card.addView(text("🎮 " + name, 18, Color.WHITE));

        Button coverButton = button("ESCOLHER CAPA");
        card.addView(coverButton);

        coverButton.setOnClickListener(v -> {
            coverIndex = index;
            openPicker(PICK_COVER, "image/*");
        });

        Button remove = button("REMOVER");
        card.addView(remove);
        remove.setOnClickListener(v -> {
            JSONArray updated = new JSONArray();

            for (int j = 0; j < games.length(); j++) {
                if (j != index) {
                    updated.put(games.opt(j));
                }
            }

            games = updated;
            save();
            render();
        });

        library.addView(card);

        View space = new View(this);
        library.addView(space,
            new LinearLayout.LayoutParams(1, dp(12)));
    }

    void openPicker(int request, String mime) {
        Intent intent = new Intent(
            Intent.ACTION_OPEN_DOCUMENT);
        intent.setType(mime);
        intent.addCategory(Intent.CATEGORY_OPENABLE);
        intent.addFlags(
            Intent.FLAG_GRANT_READ_URI_PERMISSION |
            Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION);
        startActivityForResult(intent, request);
    }

    void save() {
        prefs.edit()
            .putString("games", games.toString())
            .apply();
    }

    @Override
    protected void onActivityResult(
        int request, int result, Intent data) {

        super.onActivityResult(request, result, data);

        if (result != RESULT_OK || data == null)
            return;

        Uri uri = data.getData();
        if (uri == null) return;

        try {
            getContentResolver()
                .takePersistableUriPermission(
                    uri,
                    Intent.FLAG_GRANT_READ_URI_PERMISSION);
        } catch (Exception ignored) {}

        if (request == PICK_GAME) {
            String name = uri.getLastPathSegment();

            try (android.database.Cursor cursor =
                getContentResolver().query(
                    uri, null, null, null, null)) {

                if (cursor != null && cursor.moveToFirst()) {
                    int col = cursor.getColumnIndex(
                        android.provider.OpenableColumns.DISPLAY_NAME);
                    if (col >= 0) name = cursor.getString(col);
                }
            } catch (Exception ignored) {}

            try {
                JSONObject game = new JSONObject();
                game.put("name", name);
                game.put("uri", uri.toString());
                game.put("cover", "");
                games.put(game);
            } catch (Exception ignored) {}

        } else if (request == PICK_COVER &&
                   coverIndex >= 0) {
            try {
                games.getJSONObject(coverIndex)
                    .put("cover", uri.toString());
            } catch (Exception ignored) {}
        }

        save();
        render();
    }

    @Override
    public boolean dispatchKeyEvent(KeyEvent event) {
        if (event.getAction() == KeyEvent.ACTION_DOWN &&
            event.getDevice() != null &&
            (event.getSource() &
             android.view.InputDevice.SOURCE_GAMEPAD)
             == android.view.InputDevice.SOURCE_GAMEPAD) {

            if (controllerStatus != null) {
                controllerStatus.setText(
                    "Controle detectado: botão " +
                    event.getKeyCode());
            }
        }

        return super.dispatchKeyEvent(event);
    }

    private native String nativeVersion();

    static {
        try {
            System.loadLibrary("x360nexus");
        } catch (UnsatisfiedLinkError ignored) {}
    }
}
