# padtype for Android

A controller keyboard for Android 14 and newer, with a live DualSense preview,
the desktop independent chord layout, and editable button/stick mappings.
The app does not need root, network access, or a replacement on-screen keyboard.

## Install and use

Build the debug APK, then install it on a tablet with USB debugging enabled:

```sh
cd android
./gradlew :app:assembleDebug
adb install -r app/build/outputs/apk/debug/app-debug.apk
adb shell am start -n io.github.dlip.padtype/.MainActivity
```

1. Pair the DualSense in Android's Bluetooth settings.
2. Open **padtype → Enable controller input…**, select **padtype controller
   input**, and enable it in Android's accessibility settings. If Android blocks
   a manually installed app, use its App info menu to allow restricted settings.
3. Choose **Padtype chords** or **Direct mapping**, then **Start remapping**.
4. Focus the test field or a text field in another app. Release all controller
   controls before typing.

Pausing or stopping remapping releases joystick capture so games receive normal
controller buttons, sticks, D-pad, and analog triggers. **Pause/Resume**, or
L1 + R1 + L2 + R2, discards pending strokes. Release all four buttons between
toggles. While paused, the service observes button events without consuming new
presses, allowing the four-button shortcut to resume typing from a game.
Resuming this way requires a controller that reports L2/R2 button presses, as the
DualSense does; the app's **Resume** button also works. Buttons already held when
toggling keep their original routing through release so games receive matched
presses and releases. Release all controls after resuming typing.
The handoff waits for buttons held during the toggle to release before changing
joystick capture.
The selected controller is remembered by descriptor and reattaches automatically.
App or service restarts leave remapping off.

Assign padtype to Android's accessibility button to start or stop remapping from
any app. Each tap confirms whether controller input is on or off. Starting while
the controller is disconnected waits for it to reconnect. Release all controls
after starting input.

**Hide software keyboard while remapping** is on by default. The software keyboard
stays hidden while remapping is active and the selected controller is connected.
Pausing, stopping, disconnecting, or turning this option off restores the previous
keyboard behavior. Resuming or reconnecting hides it again. Your selected keyboard
is unchanged; focus a text field to open it normally when remapping is inactive.

The accessibility service uses Android's
[joystick event capture](https://developer.android.com/reference/android/accessibilityservice/AccessibilityService#onMotionEvent(android.view.MotionEvent))
and [accessibility input connection](https://developer.android.com/reference/android/accessibilityservice/InputMethod.AccessibilityInputConnection).
Text and keys go through the focused editor's input connection. Games and apps
without such a connection may not accept them; this app does not provide
unrestricted system-wide keyboard or touch injection. Explicit Android Back,
Home, and Recents actions are available in Direct mapping.

## Padtype chords

The included 97-binding layout matches the desktop layout at the time this app
was created. Import another version 2 **independent** `layout.json` using the
document picker. Validation happens before replacing the saved layout, and the
previous layout is kept if import fails. Export writes the current JSON to a
location you choose. Legacy unified recognition strategies are not supported on
Android yet.

**View chord reference** opens eight-way maps for the active layout, including
imported layouts. Base letters and numbers share cells with their L2 Shift
outputs (for example, `m / M` and `, / <`). Navigation, function, media, volume,
and mirrored thumbstick maps use the same format. Use the layer selector to
focus on one layer, or expand its text chord reference to see every combo.
Maps sit side by side on tablets and stack on narrower screens.

Alt + Tab switches directly to the previous app using Android's double-Recents
quick switch. Tap it again to return. Pending switches cancel when remapping
stops, pauses, or the controller disconnects.

Command + Tab opens Android's Recent Apps view, including when no text field is
focused. Release the controls, then hold R2 + L1 and use the D-pad
arrows to choose an app. Options (Enter) selects it; Share (Escape) closes the
switcher. When no editor is connected, unmodified arrows, Enter, and Escape use
Android's global navigation actions. In a text field they keep their normal
editing behavior. Plain Tab and other modified Tab combinations go to the
focused app. This opens the Android app switcher; it does not emulate holding
Alt or Command while cycling between apps on a desktop.

| DualSense control | Padtype role |
| --- | --- |
| Cross, Circle, Square, Triangle | A, B, X, Y, by physical position |
| D-pad and L1 | Independent left typing half |
| Face buttons and R1 | Independent right typing half |
| L1 alone / R1 alone | Backspace / Space; with R2, navigation / functions |
| L2 / R2 | Held Shift / numbers and symbols |
| R2 + L1 | Held navigation layer: D-pad arrows; Triangle/Cross Page Up/Down; Square/Circle Home/End |
| R2 + R1 | Held function layer: number combos 1–9 → F1–F9, 0 → F10; Cross/Square → F11/F12 |
| Options / Share | Enter / Escape in every layer |
| Sticks | Up Alt, down Command/Meta, outward Ctrl, inward Alt + Command |

Outward diagonals combine adjacent directions. Inward-up sends Command + Ctrl + Alt +
Shift; inward-down sends Command + Ctrl + Alt. Straight inward remains Alt +
Command. The layout's opposite-face-pair and forbidden-chord constraints
are enforced. Stroke output follows start order, including overlapping halves
and Options/Share. The queue is bounded at 64 pending strokes. Focus changes,
disconnects, and pauses discard pending output and require neutral controls.

With `profile.navigation_layer` enabled (as in the bundled layout), hold R2 + L1
before pressing navigation inputs. D-pad cardinal directions send arrows;
Triangle sends Page Up, Cross Page Down, Square Home, and Circle End. D-pad
arrows fire on press and repeat after 400 ms, every 80 ms. With the bundled
`profile.navigation_media` option, face-button taps send on full chord release;
holding one face button starts repeating after 400 ms. Adding a second face
button forms a media chord without leaking a page key. Media chords never
repeat. Add L2 for Shift selection; stick
modifiers also apply. D-pad diagonals produce no arrows. Releasing either R2 or
L1 stops repeat, and held navigation inputs must release before typing again.
Entering navigation discards unfinished typing strokes. L1 does not send
Backspace as a layer modifier.

R2 alone still provides numbers and symbols, including on the D-pad. Generation
reserves R2 + L1 and relocates conflicting symbol bindings. Base letters and
punctuation remain unchanged. Imported older layouts keep their original
behavior until regenerated with the navigation option enabled.

The bundled number layer uses a clockwise D-pad ring: Up is 1, Up Right is 2,
Right is 3, Down Right is 4, Down is 5, Down Left is 6, Left is 7, and Up Left
is 8. Triangle is 9; Circle is 0. L2 adds the usual shifted number symbols.
Square is `-`; Cross is `=`. Add L2 for `_` and `+`. Cross+Square is `]`
(L2 adds `}`). Number, minus, and equals positions are pinned in the example
generator configuration.

With `profile.function_layer` enabled, hold R2 + R1 and press the same ordinary
combos as numbers 1–9 for F1–F9, or 0 for F10. Cross is F11 and Square is F12 in
the bundled preset. Function keys fire once on full chord release, with L2 Shift and stick
modifiers supported. The prefix can stay held across sequential inputs. Early
prefix release preserves a function stroke already started, without sending
Space. Imported layouts use their own `profile.extra_function_chords` for F11/F12;
older layouts without the field retain D-pad Left/Down. Entering either extra
layer cancels unfinished typing; switching between
the extra layers cancels unfinished strokes and consumes held controls. Holding
R2 + L1 + R1 suppresses typing; adding L2 uses the normal pause gesture.

The bundled preset adds these adjacent face-button pairs:

| Face pair | Navigation (R2 + L1) | Functions (R2 + R1) |
| --- | --- | --- |
| Square + Triangle | Previous track | Volume up |
| Triangle + Circle | Next track | Unassigned |
| Cross + Square | Play/pause | Volume down |
| Cross + Circle | Stop | Mute/unmute |

Media and volume actions send once on full chord release and ignore keyboard
modifiers. Android dispatches media keys to the current media session and changes
the music stream volume, so these controls also work without a focused text field.
The app requests the normal `MODIFY_AUDIO_SETTINGS` permission; no runtime
permission prompt is needed. Playback commands require a media app that handles them.
The options `profile.navigation_media` and `profile.function_volume` are enabled in
the bundled preset. Older imports without them keep their original behavior;
volume combos must not overlap number or F11/F12 bindings.

Triggers engage at 65% and release at 35%; sticks engage at 55% and release at
35%. D-pad hat axes and analog trigger axes take precedence over their duplicate
button reports. DualSense right-stick input uses Android's Z/RZ axes, with RX/RY
as a fallback for other controllers. The PS button retains its Android behavior.

## Direct mappings

Each face button, shoulder, trigger, menu button, stick click, D-pad direction,
and stick direction can produce text, a keyboard key, a held modifier, an Android
navigation action, or no output. Settings are saved on the device. Defaults are:

| Input | Output |
| --- | --- |
| Left stick / D-pad | Arrow keys |
| Right stick | WASD |
| Cross / Circle / Square / Triangle | Space / Escape / Backspace / Enter |
| L1 / R1 | Tab / Space |
| L2 / R2 | Held Shift / Ctrl |
| Options / Share | Enter / Escape |

Direct mode emits a complete key tap on activation. Text, arrows, Delete, and
Backspace repeat after 400 ms, every 80 ms, while held. Releasing the control,
changing focus, pausing, or disconnecting stops repeat. Plain text supports
Unicode; modified text uses single ASCII keyboard keys. Keyboard shortcuts and
Command/Meta support depend on the receiving app.

## Build and checks

Use JDK 21 and Android SDK platform 35 with build-tools 35.0.0. Set
`ANDROID_HOME` or put `sdk.dir=…` in the ignored `android/local.properties`.
Gradle is pinned by the wrapper. On NixOS:

```sh
nix-shell android/shell.nix --run 'cd android && ./gradlew :app:assembleDebug :app:testDebugUnitTest :app:lintDebug'
```

From `android/`, run:

```sh
./gradlew :app:testDebugUnitTest :app:lintDebug
./gradlew :app:assembleDebugAndroidTest
```

The JVM and Rust tests use the same 361 recognition vectors, covering every
bundled binding, overlapping releases, rolled diagonals, held and late modifiers,
stick directions, menus, resets, forbidden pairs, startup, and queue overflow.
Run the Rust half from the repository root with `cargo test --test android`.

With the tablet and DualSense connected, run device checks after enabling the
service. The cross-app test types a fixed string into a separate editor activity
in the test APK and restores the prior mapping and mode. It leaves remapping off.
It does not read or type into personal apps:

```sh
./gradlew :app:connectedDebugAndroidTest
```

The device checks cover Android's own JSON implementation, actual DualSense axis
capabilities, duplicate trigger reports, and text/key delivery through a separate
application's input connection. The controller event sequences in the automated
cross-app test are injected by Android's testing API; testing with physical
button presses remains a separate step.

The separate gamepad check preserves the running accessibility service and
uses Android's shell `uinput` tool to create a temporary DualSense-like gamepad.
It checks pause/resume through the four layer buttons, both sticks, analog
triggers, D-pad axes, balanced button releases, and typing after resuming.
It restores the selected controller, removes the virtual pad, and leaves
remapping off. Start with the bundled R1 Space mapping and a connected controller:

```sh
./gradlew :gamepad-check:assembleDebug
adb install -r gamepad-check/build/outputs/apk/debug/gamepad-check-debug.apk
adb shell am instrument -w io.github.dlip.padtype.gamepadcheck/io.github.dlip.padtype.GamepadCheck
```

The production manifest requests no internet or storage permissions and no
screen-content retrieval. Imported layouts and mappings remain in private app
preferences; typed text is neither stored nor logged. The last output indicator
is held in memory until the app process ends. Debug APKs use the local debug
signing key and are intended for sideloading, not store distribution.
