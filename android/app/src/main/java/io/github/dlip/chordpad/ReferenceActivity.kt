package io.github.dlip.chordpad

import android.app.Activity
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.os.Bundle
import android.view.Gravity
import android.view.View
import android.view.WindowInsets
import android.widget.AdapterView
import android.widget.ArrayAdapter
import android.widget.Button
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.Spinner
import android.widget.TextView
import kotlin.math.roundToInt

class ReferenceActivity : Activity() {
    private val ink = 0xff182b34.toInt()
    private val muted = 0xff586a73.toInt()
    private val blue = 0xff1557b0.toInt()
    private val sections = mutableListOf<View>()
    private lateinit var filter: Spinner

    override fun onCreate(bundle: Bundle?) {
        super.onCreate(bundle)
        val root = column().apply { setBackgroundColor(0xfff6f5f0.toInt()) }
        setContentView(root)
        root.setOnApplyWindowInsetsListener { _, insets ->
            val bars = insets.getInsets(WindowInsets.Type.systemBars() or WindowInsets.Type.displayCutout())
            root.setPadding(bars.left, bars.top, bars.right, bars.bottom)
            insets
        }
        val toolbar = LinearLayout(this).apply {
            gravity = Gravity.CENTER_VERTICAL; setPadding(dp(16), dp(8), dp(16), 0)
        }
        root.addView(toolbar)
        toolbar.addView(text("Chord reference", 24f, ink).apply { setTypeface(null, Typeface.BOLD) }, LinearLayout.LayoutParams(0, -2, 1f))
        toolbar.addView(button("Done").apply { setOnClickListener { finish() } })
        val layers = LayoutReference.layers(AppState.get(this).layout)
        val choices = listOf("All layers") + layers.map { it.title } + "Thumbstick modifiers"
        filter = Spinner(this).apply {
            contentDescription = "Reference layer"
            adapter = ArrayAdapter(this@ReferenceActivity, android.R.layout.simple_spinner_item, choices).apply {
                setDropDownViewResource(android.R.layout.simple_spinner_dropdown_item)
            }
        }
        root.addView(filter, LinearLayout.LayoutParams(-1, dp(56)).apply { setMargins(dp(16), 0, dp(16), 0) })
        val scroll = ScrollView(this); root.addView(scroll, LinearLayout.LayoutParams(-1, 0, 1f))
        val page = column().apply { setPadding(dp(16), 0, dp(16), dp(24)) }; scroll.addView(page)
        page.addView(text("A / Cross × · B / Circle ○ · X / Square □ · Y / Triangle △\nOptions: Enter · Share: Escape in every layer. Hold modifiers before starting a chord; release ordinary controls to type.", 14f, muted))
        for (layer in layers) {
            val section = section(page, layer.title, layer.note)
            maps(section, layer.maps, false)
            val toggle = button("Show text chord reference"); section.addView(toggle)
            val details = column().apply { visibility = View.GONE }; section.addView(details)
            for (chord in layer.chords) {
                details.addView(text("${chord.output}    ←    ${chord.controls}", 16f, ink).apply {
                    setTextIsSelectable(true); setPadding(0, dp(8), 0, dp(8))
                })
            }
            toggle.setOnClickListener {
                val show = details.visibility != View.VISIBLE
                details.visibility = if (show) View.VISIBLE else View.GONE
                toggle.text = if (show) "Hide text chord reference" else "Show text chord reference"
            }
        }
        val sticks = section(page, "Thumbstick modifiers", "Hold either stick while typing. Inward points toward the controller center. Command sends Meta / Super. Stick modifiers combine with L2 Shift and R2 layers.")
        maps(sticks, LayoutReference.sticks(), true)
        filter.onItemSelectedListener = object : AdapterView.OnItemSelectedListener {
            override fun onItemSelected(parent: AdapterView<*>?, view: View?, position: Int, id: Long) {
                sections.forEachIndexed { i, section -> section.visibility = if (position == 0 || position == i + 1) View.VISIBLE else View.GONE }
                scroll.scrollTo(0, 0)
            }
            override fun onNothingSelected(parent: AdapterView<*>?) {}
        }
        if (bundle != null) filter.setSelection(bundle.getInt("layer", 0))
    }

    override fun onSaveInstanceState(bundle: Bundle) {
        bundle.putInt("layer", filter.selectedItemPosition)
        super.onSaveInstanceState(bundle)
    }

    private fun section(page: LinearLayout, title: String, note: String): LinearLayout {
        val section = column().apply { setPadding(0, dp(20), 0, 0) }
        page.addView(section); sections.add(section)
        section.addView(text(title, 22f, ink).apply { setTypeface(null, Typeface.BOLD); isAccessibilityHeading = true })
        section.addView(text(note, 15f, muted))
        return section
    }

    private fun maps(section: LinearLayout, maps: List<LayoutReference.Map>, sticks: Boolean) {
        val config = resources.configuration
        val columns = if (config.screenWidthDp / config.fontScale >= 720) 2 else 1
        for (i in maps.indices step columns) {
            val row = LinearLayout(this).apply { layoutDirection = View.LAYOUT_DIRECTION_LTR; isBaselineAligned = false }
            section.addView(row)
            for (j in i until minOf(i + columns, maps.size)) {
                val map = maps[j]
                val panel = column()
                row.addView(panel, LinearLayout.LayoutParams(0, -2, 1f).apply { if (j != i) leftMargin = dp(16) })
                panel.addView(text(map.title, 18f, ink).apply { setTypeface(null, Typeface.BOLD); isAccessibilityHeading = true })
                for (r in 0..2) {
                    val cells = LinearLayout(this).apply { isBaselineAligned = false }; panel.addView(cells)
                    for (c in 0..2) {
                        val cell = map.cells[r * 3 + c]
                        val box = column().apply {
                            gravity = Gravity.CENTER; setPadding(dp(6), dp(10), dp(6), dp(10)); minimumHeight = dp(94)
                            background = GradientDrawable().apply { setColor(0xfff6f5f0.toInt()); cornerRadius = dp(8).toFloat(); setStroke(dp(1), 0xffd7dfdf.toInt()) }
                        }
                        val output = text(if (sticks) cell.controls else cell.output, if (sticks) 24f else 20f, blue).apply {
                            gravity = Gravity.CENTER; setTypeface(null, Typeface.BOLD)
                        }
                        box.addView(output, LinearLayout.LayoutParams(-1, -2))
                        val controls = text(if (sticks) cell.output else cell.controls, if (sticks) 14f else 12f, ink).apply { gravity = Gravity.CENTER }
                        box.addView(controls, LinearLayout.LayoutParams(-1, -2))
                        box.contentDescription = "${map.title}: ${cell.controls}, ${if (cell.output == "·") "Unassigned" else cell.output}"
                        box.importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_YES
                        output.importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
                        controls.importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
                        cells.addView(box, LinearLayout.LayoutParams(0, -1, 1f).apply { setMargins(0, 0, if (c == 2) 0 else dp(6), dp(6)) })
                    }
                }
            }
        }
    }

    private fun column() = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL }
    private fun text(value: String, size: Float, color: Int) = TextView(this).apply {
        text = value; textSize = size; setTextColor(color); setPadding(0, dp(4), 0, dp(8))
    }
    private fun button(label: String) = Button(this).apply { text = label; isAllCaps = false; setTextColor(blue); minHeight = dp(48) }
    private fun dp(value: Int) = (value * resources.displayMetrics.density).roundToInt()
}
