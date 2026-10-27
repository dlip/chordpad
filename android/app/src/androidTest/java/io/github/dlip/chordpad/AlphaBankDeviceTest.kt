@file:Suppress("DEPRECATION")
package io.github.dlip.chordpad

import android.test.InstrumentationTestCase
import org.json.JSONArray

class AlphaBankDeviceTest : InstrumentationTestCase() {
    fun testLinuxRecognitionVectorsOnTablet() {
        val layout = instrumentation.targetContext.assets.open("default-layout.json").bufferedReader().use { it.readText() }
        val cases = instrumentation.context.assets.open("alpha-bank-vectors.json").bufferedReader().use { JSONArray(it.readText()) }
        assertEquals(cases.length(), RecognitionVectors.check(layout, cases))
        val layers = LayoutReference.layers(ChordLayout(layout))
        assertEquals(10, layers.size)
        assertEquals("Volume Up", layers[7].maps[0].cells[1].output)
        assertEquals("Command + D", LayoutReference.sticks()[0].cells[4].output)
    }
}
