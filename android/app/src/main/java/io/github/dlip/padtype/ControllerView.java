package io.github.dlip.padtype;

import android.content.Context;
import android.graphics.Canvas;
import android.graphics.Paint;
import android.graphics.Path;
import android.view.View;
import static io.github.dlip.padtype.ChordLayout.*;

public final class ControllerView extends View {
    private final AppState state;
    private final Paint paint = new Paint(Paint.ANTI_ALIAS_FLAG);
    private final Path shell = new Path();
    public ControllerView(Context context) { this(context,AppState.get(context)); }
    public ControllerView(Context context, AppState state) {
        super(context); this.state = state; setContentDescription("Live controller input");
        shell.moveTo(92,75); shell.quadTo(79,52,54,66); shell.lineTo(17,203); shell.quadTo(9,238,41,243);
        shell.quadTo(64,249,104,203); shell.lineTo(133,179); shell.lineTo(347,179); shell.lineTo(376,203);
        shell.quadTo(416,249,439,243); shell.quadTo(471,238,463,203); shell.lineTo(426,66);
        shell.quadTo(401,52,388,75); shell.close();
    }
    @Override protected void onMeasure(int width, int height) {
        int w = MeasureSpec.getSize(width);
        setMeasuredDimension(w, Math.min(Math.round(w * 0.57f),Math.round(210*getResources().getDisplayMetrics().density)));
    }
    @Override protected void onDraw(Canvas canvas) {
        super.onDraw(canvas);
        canvas.save();
        float scale = Math.min(getWidth()/480f,getHeight()/270f);
        canvas.translate((getWidth()-480*scale)/2,0); canvas.scale(scale,scale);
        Frame frame = state.frame();
        paint.setStyle(Paint.Style.FILL); paint.setColor(0xffe9efef); canvas.drawPath(shell,paint);
        paint.setStyle(Paint.Style.STROKE); paint.setStrokeWidth(1.5f); paint.setColor(0xffc2d0d0); canvas.drawPath(shell,paint); paint.setStyle(Paint.Style.FILL);
        control(canvas, 50, 6, 115, 25, "L2", (frame.buttons() & L2) != 0);
        control(canvas, 315, 6, 115, 25, "R2", (frame.buttons() & R2) != 0);
        control(canvas, 50, 40, 115, 25, "L1", (frame.buttons() & L1) != 0);
        control(canvas, 315, 40, 115, 25, "R1", (frame.buttons() & R1) != 0);
        if (state.input != null) {
            paint.setColor(0xff1557b0); canvas.drawRoundRect(50,33,50+115*state.input.l2,36,2,2,paint);
            canvas.drawRoundRect(315,33,315+115*state.input.r2,36,2,2,paint);
        }
        circle(canvas, 372, 183, 23, "×", (frame.buttons() & A) != 0);
        circle(canvas, 418, 137, 23, "○", (frame.buttons() & B) != 0);
        circle(canvas, 326, 137, 23, "□", (frame.buttons() & ChordLayout.X) != 0);
        circle(canvas, 372, 91, 23, "△", (frame.buttons() & ChordLayout.Y) != 0);
        control(canvas, 91,85,34,34,"↑",(frame.dpad()&4)!=0);
        control(canvas, 91,153,34,34,"↓",(frame.dpad()&8)!=0);
        control(canvas, 57,119,34,34,"←",(frame.dpad()&1)!=0);
        control(canvas, 125,119,34,34,"→",(frame.dpad()&2)!=0);
        control(canvas, 186,104,45,22,"Share",(frame.menu()&2)!=0);
        control(canvas, 249,104,45,22,"Options",(frame.menu()&1)!=0);
        stick(canvas, 184, 186, "Left", state.input == null ? 0 : state.input.leftX, state.input == null ? 0 : state.input.leftY);
        stick(canvas, 278, 186, "Right", state.input == null ? 0 : state.input.rightX, state.input == null ? 0 : state.input.rightY);
        canvas.restore();
    }
    private void control(Canvas canvas,float x,float y,float w,float h,String label,boolean active) {
        paint.setColor(active ? 0xff1557b0 : 0xfffefefd); canvas.drawRoundRect(x,y,x+w,y+h,7,7,paint);
        label(canvas,label,x+w/2,y+h/2+4,Math.min(15,h*.5f),active ? 0xffffffff : 0xff182b34);
    }
    private void circle(Canvas canvas,float x,float y,float radius,String text,boolean active) {
        paint.setColor(active ? 0xff1557b0 : 0xfffefefd); canvas.drawCircle(x,y,radius,paint);
        label(canvas,text,x,y+7,26,active ? 0xffffffff : 0xff182b34);
    }
    private void stick(Canvas canvas,float x,float y,String name,float dx,float dy) {
        paint.setColor(0xffc7d4d7); canvas.drawCircle(x,y,25,paint);
        paint.setColor(dx*dx+dy*dy>.12f ? 0xff1557b0 : 0xff586a73); canvas.drawCircle(x+dx*12,y+dy*12,13,paint);
        label(canvas,name,x,y+44,11,0xff586a73);
    }
    private void label(Canvas canvas,String text,float x,float y,float size,int color) {
        paint.setColor(color); paint.setTextSize(size); paint.setTextAlign(Paint.Align.CENTER); canvas.drawText(text,x,y,paint);
    }
}
