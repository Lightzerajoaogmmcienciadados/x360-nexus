package com.x360nexus

import android.content.Context
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.view.MotionEvent
import android.view.View
import kotlin.math.hypot

/** Two-axis touchscreen control. Coordinates are mapped to signed gamepad axes. */
class AnalogPad(context: Context, attrs: android.util.AttributeSet?) : View(context,attrs) {
    var onAxis: ((Int, Int) -> Unit)? = null
    private val paint=Paint(Paint.ANTI_ALIAS_FLAG)
    private var x=0f
    private var y=0f
    private fun axis(value:Float):Int=(value.coerceIn(-1f,1f)*32767f).toInt()
    override fun onDraw(canvas:Canvas){
        super.onDraw(canvas)
        val cx=width/2f;val cy=height/2f
        val radius=(minOf(width,height)*0.43f).coerceAtLeast(4f)
        paint.style=Paint.Style.STROKE;paint.strokeWidth=3f;paint.color=Color.argb(200,46,204,113)
        canvas.drawCircle(cx,cy,radius,paint)
        paint.style=Paint.Style.FILL;paint.color=Color.argb(190,16,124,16)
        canvas.drawCircle(cx+x*radius*0.62f,cy+y*radius*0.62f,radius*0.40f,paint)
    }
    override fun onTouchEvent(event:MotionEvent):Boolean{
        when(event.actionMasked){
            MotionEvent.ACTION_DOWN,MotionEvent.ACTION_MOVE -> {
                val radius=(minOf(width,height)*0.43f).coerceAtLeast(4f)
                val dx=(event.x-width/2f)/radius
                val dy=(event.y-height/2f)/radius
                val length=hypot(dx,dy).coerceAtLeast(1f)
                x=dx/length;y=dy/length
                onAxis?.invoke(axis(x),axis(y));invalidate();return true
            }
            MotionEvent.ACTION_UP,MotionEvent.ACTION_CANCEL -> {
                x=0f;y=0f;onAxis?.invoke(0,0);invalidate();return true
            }
        }
        return true
    }
}