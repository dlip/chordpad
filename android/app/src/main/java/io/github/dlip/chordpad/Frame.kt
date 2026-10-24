package io.github.dlip.chordpad

data class Frame(
    val buttons: Int, val dpad: Int, val modifiers: Int, val menu: Int,
    val leftX: Int, val leftY: Int, val rightX: Int, val rightY: Int
) {
    fun neutral() = buttons == 0 && dpad == 0 && menu == 0 && modifiers == 0 &&
        leftX == 0 && leftY == 0 && rightX == 0 && rightY == 0

    fun valid() = dpad and 3 != 3 && dpad and 12 != 12

    companion object {
        val NEUTRAL = Frame(0, 0, 0, 0, 0, 0, 0, 0)
    }
}
