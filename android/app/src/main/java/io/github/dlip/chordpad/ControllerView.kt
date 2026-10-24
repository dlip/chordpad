package io.github.dlip.chordpad

import android.content.Context
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.Path
import android.view.View
import kotlin.math.roundToInt

class ControllerView @JvmOverloads constructor(context: Context, private val state: AppState = AppState.get(context)) : View(context) {
    private val paint = Paint(Paint.ANTI_ALIAS_FLAG)
    private val shell = Path().apply {
        moveTo(92f, 75f); quadTo(79f, 52f, 54f, 66f); lineTo(17f, 203f); quadTo(9f, 238f, 41f, 243f)
        quadTo(64f, 249f, 104f, 203f); lineTo(133f, 179f); lineTo(347f, 179f); lineTo(376f, 203f)
        quadTo(416f, 249f, 439f, 243f); quadTo(471f, 238f, 463f, 203f); lineTo(426f, 66f)
        quadTo(401f, 52f, 388f, 75f); close()
    }
    init { contentDescription = "Live controller input" }

    override fun onMeasure(widthMeasureSpec: Int, heightMeasureSpec: Int) {
        val w = MeasureSpec.getSize(widthMeasureSpec)
        setMeasuredDimension(w, minOf((w * .57f).roundToInt(), (210 * resources.displayMetrics.density).roundToInt()))
    }

    override fun onDraw(canvas: Canvas) {
        super.onDraw(canvas)
        canvas.save()
        val scale = minOf(width / 480f, height / 270f)
        canvas.translate((width - 480 * scale) / 2, 0f)
        canvas.scale(scale, scale)
        val frame = state.frame()
        paint.style = Paint.Style.FILL; paint.color = 0xffe9efef.toInt(); canvas.drawPath(shell, paint)
        paint.style = Paint.Style.STROKE; paint.strokeWidth = 1.5f; paint.color = 0xffc2d0d0.toInt()
        canvas.drawPath(shell, paint); paint.style = Paint.Style.FILL
        control(canvas, 50f, 6f, 115f, 25f, "L2", frame.buttons and L2 != 0)
        control(canvas, 315f, 6f, 115f, 25f, "R2", frame.buttons and R2 != 0)
        control(canvas, 50f, 40f, 115f, 25f, "L1", frame.buttons and L1 != 0)
        control(canvas, 315f, 40f, 115f, 25f, "R1", frame.buttons and R1 != 0)
        state.input?.let {
            paint.color = 0xff1557b0.toInt()
            canvas.drawRoundRect(50f, 33f, 50 + 115 * it.l2, 36f, 2f, 2f, paint)
            canvas.drawRoundRect(315f, 33f, 315 + 115 * it.r2, 36f, 2f, 2f, paint)
        }
        circle(canvas, 372f, 183f, 23f, "×", frame.buttons and A != 0)
        circle(canvas, 418f, 137f, 23f, "○", frame.buttons and B != 0)
        circle(canvas, 326f, 137f, 23f, "□", frame.buttons and io.github.dlip.chordpad.X != 0)
        circle(canvas, 372f, 91f, 23f, "△", frame.buttons and io.github.dlip.chordpad.Y != 0)
        control(canvas, 91f, 85f, 34f, 34f, "↑", frame.dpad and 4 != 0)
        control(canvas, 91f, 153f, 34f, 34f, "↓", frame.dpad and 8 != 0)
        control(canvas, 57f, 119f, 34f, 34f, "←", frame.dpad and 1 != 0)
        control(canvas, 125f, 119f, 34f, 34f, "→", frame.dpad and 2 != 0)
        control(canvas, 186f, 104f, 45f, 22f, "Share", frame.menu and 2 != 0)
        control(canvas, 249f, 104f, 45f, 22f, "Options", frame.menu and 1 != 0)
        stick(canvas, 184f, 186f, "Left", state.input?.leftX ?: 0f, state.input?.leftY ?: 0f)
        stick(canvas, 278f, 186f, "Right", state.input?.rightX ?: 0f, state.input?.rightY ?: 0f)
        canvas.restore()
    }

    private fun control(canvas: Canvas, x: Float, y: Float, w: Float, h: Float, text: String, active: Boolean) {
        paint.color = (if (active) 0xff1557b0 else 0xfffefefd).toInt()
        canvas.drawRoundRect(x, y, x + w, y + h, 7f, 7f, paint)
        label(canvas, text, x + w / 2, y + h / 2 + 4, minOf(15f, h * .5f), (if (active) 0xffffffff else 0xff182b34).toInt())
    }
    private fun circle(canvas: Canvas, x: Float, y: Float, radius: Float, text: String, active: Boolean) {
        paint.color = (if (active) 0xff1557b0 else 0xfffefefd).toInt()
        canvas.drawCircle(x, y, radius, paint)
        label(canvas, text, x, y + 7, 26f, (if (active) 0xffffffff else 0xff182b34).toInt())
    }
    private fun stick(canvas: Canvas, x: Float, y: Float, name: String, dx: Float, dy: Float) {
        paint.color = 0xffc7d4d7.toInt(); canvas.drawCircle(x, y, 25f, paint)
        paint.color = (if (dx * dx + dy * dy > .12f) 0xff1557b0 else 0xff586a73).toInt()
        canvas.drawCircle(x + dx * 12, y + dy * 12, 13f, paint)
        label(canvas, name, x, y + 44, 11f, 0xff586a73.toInt())
    }
    private fun label(canvas: Canvas, text: String, x: Float, y: Float, size: Float, color: Int) {
        paint.color = color; paint.textSize = size; paint.textAlign = Paint.Align.CENTER
        canvas.drawText(text, x, y, paint)
    }
}
