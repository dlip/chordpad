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

Stopping remapping releases joystick capture. Pause/Resume, or
L1 + R1 + L2 + R2, discards pending strokes. Pausing keeps controller capture
active; use **Stop remapping** to give joystick input back to other apps.
The selected controller is remembered by descriptor and reattaches automatically.
App or service restarts leave remapping off.

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

| DualSense control | Padtype role |
| --- | --- |
| Cross, Circle, Square, Triangle | A, B, X, Y, by physical position |
| D-pad and L1 | Independent left typing half |
| Face buttons and R1 | Independent right typing half |
| L1 alone / R1 alone | Backspace / Space |
| L2 / R2 | Held Shift / numbers and symbols |
| Options / Share | Enter / Escape in every layer |
| Sticks | Up Alt, down Command/Meta, outward Ctrl, inward Alt + Command |

Diagonals combine adjacent directions. Inward stick diagonals are also
Alt + Command. The layout's opposite-face-pair and forbidden-chord constraints
are enforced. Stroke output follows start order, including overlapping halves
and Options/Share. The queue is bounded at 64 pending strokes. Focus changes,
disconnects, and pauses discard pending output and require neutral controls.

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

The JVM and Rust tests use the same 146 recognition vectors, covering every
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

The production manifest requests no internet or storage permissions and no
screen-content retrieval. Imported layouts and mappings remain in private app
preferences; typed text is neither stored nor logged. The last output indicator
is held in memory until the app process ends. Debug APKs use the local debug
signing key and are intended for sideloading, not store distribution.
