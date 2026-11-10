
package com.x360nexus;

import android.app.Activity;
import android.os.Bundle;
import android.graphics.Color;
import android.view.View;
import android.widget.*;
import android.content.Intent;
import android.net.Uri;

public class MainActivity extends Activity {

    LinearLayout layout;
    TextView status;

    @Override
    public void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);

        layout = new LinearLayout(this);
        layout.setOrientation(LinearLayout.VERTICAL);
        layout.setPadding(35, 50, 35, 25);
        layout.setBackgroundColor(Color.rgb(10, 15, 12));

        TextView title = new TextView(this);
        title.setText("X360 NEXUS");
        title.setTextSize(32);
        title.setTextColor(Color.rgb(20, 220, 90));
        layout.addView(title);

        TextView subtitle = new TextView(this);
        subtitle.setText("Xbox 360 Gaming Hub");
        subtitle.setTextColor(Color.WHITE);
        layout.addView(subtitle);

        Button importButton = new Button(this);
        importButton.setText("IMPORTAR JOGO");
        layout.addView(importButton);

        TextView library = new TextView(this);
        library.setText("BIBLIOTECA DE JOGOS");
        library.setTextSize(22);
        library.setTextColor(Color.WHITE);
        layout.addView(library);

        status = new TextView(this);
        status.setText("Nenhum jogo importado");
        status.setTextColor(Color.LTGRAY);
        layout.addView(status);

        importButton.setOnClickListener(v -> {
            Intent intent = new Intent(Intent.ACTION_OPEN_DOCUMENT);
            intent.setType("*/*");
            intent.addCategory(Intent.CATEGORY_OPENABLE);
            startActivityForResult(intent, 100);
        });

        setContentView(layout);
    }

    @Override
    protected void onActivityResult(
        int requestCode,
        int resultCode,
        Intent data
    ) {
        super.onActivityResult(requestCode, resultCode, data);

        if (requestCode == 100 &&
            resultCode == RESULT_OK &&
            data != null) {

            Uri uri = data.getData();

            if (uri != null) {
                status.setText("Arquivo selecionado:\n" + uri);
            }
        }
    }
}
