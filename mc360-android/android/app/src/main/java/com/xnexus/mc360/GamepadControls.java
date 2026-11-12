package com.xnexus.mc360;
import android.view.KeyEvent;
public final class GamepadControls {
    private GamepadControls() {}
    public static float yawDelta(int keyCode) {
        if (keyCode == KeyEvent.KEYCODE_DPAD_LEFT || keyCode == KeyEvent.KEYCODE_BUTTON_L1) return -0.13f;
        if (keyCode == KeyEvent.KEYCODE_DPAD_RIGHT || keyCode == KeyEvent.KEYCODE_BUTTON_R1) return 0.13f;
        return 0f;
    }
}
