package io.github.dlip.padtype;

public record Frame(int buttons, int dpad, int modifiers, int menu,
                    int leftX, int leftY, int rightX, int rightY) {
    public static final Frame NEUTRAL = new Frame(0, 0, 0, 0, 0, 0, 0, 0);
    public boolean neutral() {
        return buttons == 0 && dpad == 0 && menu == 0 && modifiers == 0
                && leftX == 0 && leftY == 0 && rightX == 0 && rightY == 0;
    }
    public boolean valid() { return (dpad & 3) != 3 && (dpad & 12) != 12; }
}
