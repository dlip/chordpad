# Chordpad for Android

A controller keyboard for Android 14 and newer, with a live DualSense preview,
the four-bank typing layout, and editable button/stick mappings.
The app does not need root, network access, or a replacement on-screen keyboard.

## Install and use

Download `chordpad.apk` from the [latest GitHub release](https://github.com/dlip/chordpad/releases/latest)
and open it on your Android device to install. Allow installation from the app
you used to download it if Android asks.

To build it yourself, build the debug APK and install it with USB debugging enabled:

```sh
cd android
./gradlew :app:assembleDebug
adb install -r app/build/outputs/apk/debug/app-debug.apk
adb shell am start -n io.github.dlip.chordpad/.MainActivity
```

The Android application ID is `io.github.dlip.chordpad`. This installs separately
from builds using the earlier package. Export your layout from the earlier app
and import it into Chordpad; direct mappings and controller preferences need to
be configured again. Stop remapping in the earlier app before enabling
Chordpad's controller input service in Android accessibility settings.

1. Pair the DualSense in Android's Bluetooth settings.
2. Open **Chordpad → Enable controller input…**, select **Chordpad controller
   input**, and enable it in Android's accessibility settings. If Android blocks
   a manually installed app, use its App info menu to allow restricted settings.
3. Choose **Chordpad chords** or **Direct mapping**, then **Start remapping**.
4. Focus the test field or a text field in another app. Release all controller
   controls before typing.

Pausing or stopping remapping releases joystick capture so games receive normal
controller buttons, sticks, D-pad, and analog triggers. **Pause/Resume**, or
A + B + X + Y, discards pending strokes. Release all four buttons between
toggles. While paused, the service observes button events without consuming new
presses, allowing the four-button shortcut to resume typing from a game.
The face-button gesture works without trigger button reports; the app's
**Resume** button also works. Buttons already held when
toggling keep their original routing through release so games receive matched
presses and releases. Release all controls after resuming typing.
The handoff waits for buttons held during the toggle to release before changing
joystick capture.
The selected controller is remembered by descriptor and reattaches automatically.
App or service restarts leave remapping off.

Assign Chordpad to Android's accessibility button to start or stop remapping from
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
Home, and Recents actions are available in Direct mapping. Modifier keys apply to Chordpad's typed keys. Held Shift also extends a
selection when you click or tap elsewhere in the same focused text editor.
Native and browser editors must support Android's editor selection updates and
selection commands. Chordpad uses cursor positions supplied by the editor;
it does not record the text or send it anywhere.

## Global mouse modifiers

For Ctrl-click, Alt-click, Meta-click, and native Shift mouse behavior in other
apps, download `start-modifiers.sh` from the [latest release](https://github.com/dlip/chordpad/releases/latest).
Connect the tablet to a computer with ADB and USB debugging enabled, then run:

```sh
bash start-modifiers.sh
# With more than one device: bash start-modifiers.sh -s SERIAL
```

From a source checkout, use `./android/start-modifiers.sh` instead.

The app shows **Global mouse modifiers connected**. Disconnect USB and use
held L2, stick modifiers, or Direct mapping modifiers with your mouse. The
helper needs restarting after a tablet reboot, force-stopping Chordpad, or
updating the app. Ordinary typing and editor Shift range selection work
without it. Unused L2 taps remain one-shot modifiers for the next typed key;
hold L2 for a mouse or touch selection.

The helper runs with ADB privileges and creates a virtual modifier keyboard.
It accepts only modifier state from Chordpad's UID, releases keys on pause,
stop, reset, or controller disconnect, and clears them if the app stops sending
heartbeats for 1.5 seconds (checked every 500 ms). App termination closes the helper and its keyboard.
It requires the device's `uinput` command and Android shell access to virtual
input devices; vendor restrictions may prevent startup. Apps still decide how
to interpret modifiers outside focused text editors, including list and file
range selection.

## Chordpad chords

The included [four-bank layout](../layout.json) uses
one D-pad cardinal or one face button per character. Open **View chord reference**
for the active banks, shifted symbols, navigation, functions, and mirrored
thumbstick maps. The [one-page PDF](../layout.pdf)
shows the same layout.

| Control | Action |
| --- | --- |
| L1 / R1 held | Select alpha banks; both select the fourth bank |
| L1 / R1 alone | Backspace / Space; hold alone 300 ms to repeat |
| L2 held / tapped | Shift / one-shot Shift; tap again to cancel |
| R2 held or tapped once | Numbers and symbols; D-pad ↑ → ↓ ← gives 1–4 |
| R2 + L1 | D-pad 5–8; R2 A is 9 and B is 0 |
| R1 + R2 | Navigation: D-pad arrows; Y Page Up, A Page Down, X Home, B End |
| R1 + R2 + L1 | Editing/media: ↑/↓ volume, ←/→ previous/next track, A Play/Pause, B Stop, Y Print Screen, X Delete |
| Tap R2 twice; third tap turns off | Next key uses function positions; add L1 for F5–F8 |
| Options / Share | Enter / Escape in every layer |
| L3 / R3 click | Command+D / Command+Shift+D |
| A + B + X + Y | Pause / resume |

Hold bank bumpers before a thumb button, or add them within **100 ms** while that
button stays down. Only the newest stroke accepts late bumpers. Starting another
thumb button freezes earlier strokes; early bumper releases preserve captured
banks. Adjacent buttons can overlap to roll letters. Each press is a separate
stroke, and output follows press order even if releases arrive in reverse.

Unused R2 taps cycle numbers → functions → off. Each tap lasts at most 250 ms,
with no limit between taps. The next key clears the selection. Holding R1+R2
selects navigation. Unused L2 taps of at most 250 ms arm Shift for the next key;
a second tap cancels it. Used holds do not rearm either selection. Pause, disconnect, and
focus resets clear both.

Held Shift combines with navigation for selection and is captured at stroke
start. An unused L2 tap shifts the next key only. Standard US shifted
symbols and uppercase letters are supported. Caps Lock is not implemented.

Tilt either stick for mirrored keyboard modifiers: up Alt, down Command/Meta,
outward Ctrl, inward Ctrl+Alt+Command, inward-up Ctrl+Alt+Shift+Command, and
inward-down Alt+Command. Click shortcuts ignore held modifiers. Short taps send once on release; holding
the click repeats its shortcut. On your computer you can assign L3's Command+D to Dictate and R3's
Command+Shift+D to Cancel Dictation; these are keyboard shortcuts, not Android
speech-recognition actions.

A standalone L1/Backspace or R1/Space hold starts repeating after **300 ms**,
every **80 ms**. Another control pressed before that deadline prevents repeat
for that hold. Once repeating, the bumper acts as its key until release and no
longer selects a bank. Preheld Shift/stick modifiers still apply. Release stops
repeat without an extra tap; pause, disconnect, and focus reset stop it too.

Every mapped output in the four-bank layout repeats after a **300 ms** hold,
then every **80 ms**: letters, numbers, function/navigation keys, media,
Enter/Escape, and stick-click shortcuts. Layer and modifier selectors remain
held selectors. First outputs keep press order; a repeating key does not block
later completed strokes. Release stops that key without an extra tap. Captured
banks, modes, and modifiers stay fixed throughout the hold.

Media controls ignore keyboard modifiers. Android dispatches media keys to
the current media session and changes music volume without needing an editor.
Unmodified Print Screen takes a screenshot; modified Print Screen goes to the
focused editor.

Alt+Tab switches to the previous app. Command+Tab opens Recent Apps; navigate with
R1+R2 arrows, then Options to select or Share to close. Plain Tab and other
modified Tab combinations go to the focused app. Without an editor, unmodified
arrows, Enter, and Escape use Android's global navigation actions.

Import four-bank JSON or a version 2 independent legacy layout
using **Import layout…**. Validation completes before replacing the saved layout;
failed imports leave it intact. Legacy layouts keep their original chord,
navigation, and function mappings. **Restore bundled layout** selects the new
four-bank layout. Export saves the active JSON. Reference maps always follow the
active layout, with a layer selector and optional text instructions.

Focus changes, pause, disconnect, and resynchronization discard pending strokes
and require all controls to return to neutral. The queue is bounded at 64 strokes.
Triggers engage at 65% and release at 35%; sticks engage at 55% and release at
35%. Hat and analog trigger axes take precedence over duplicate button reports.
DualSense uses Z/RZ for its right stick, with RX/RY as a fallback. The PS button
retains Android behavior.

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
The app, tests, and gamepad check are written in Kotlin; Gradle supplies the
compiler. Gradle is pinned by the wrapper. On NixOS:

```sh
nix-shell android/shell.nix --run 'cd android && ./gradlew :app:assembleDebug :app:testDebugUnitTest :app:lintDebug'
```

From `android/`, run:

```sh
./gradlew :app:testDebugUnitTest :app:lintDebug
./gradlew :app:assembleDebugAndroidTest
```

The JVM and Rust tests use the same 371 recognition vectors, covering every
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
adb shell am instrument -w io.github.dlip.chordpad.gamepadcheck/io.github.dlip.chordpad.GamepadCheck
```

The production manifest requests no internet or storage permissions and no
screen-content retrieval. Imported layouts and mappings remain in private app
preferences; typed text is neither stored nor logged. The last output indicator
is held in memory until the app process ends. Debug APKs use the local debug
signing key and are intended for sideloading, not store distribution.

## Tagged releases

The [Android release workflow](../.github/workflows/android-release.yml) runs on
every pushed tag. It runs release unit tests and lint, builds and signs
`chordpad.apk`, and publishes it in a GitHub release with notes from
[CHANGELOG.md](../CHANGELOG.md). A tag such as `v1.0.1` uses the changes below
`## 1.0.1`, up to the next version heading. Missing, empty, or duplicate entries
stop the workflow before it builds the APK.
Rerunning a successful workflow replaces the APK and refreshes its release notes,
unless release immutability is enabled in the repository.

The APK's version name is the tag. Its version code is the workflow run number
plus one, so each new run produces an increasing version code and reruns keep
the same code. Keep the workflow file's name when updating it to retain its run
counter.

Before pushing the first tag, configure these repository secrets under
**Settings → Secrets and variables → Actions**:

| Secret | Value |
| --- | --- |
| `ANDROID_KEYSTORE_BASE64` | Base64-encoded release keystore |
| `ANDROID_KEYSTORE_PASSWORD` | Keystore password |
| `ANDROID_KEY_ALIAS` | Signing key alias |
| `ANDROID_KEY_PASSWORD` | Signing key password; use the keystore password for PKCS12 |

Reuse your existing release key if you have one. To create a new key, run this
outside the repository and keep a backup of the keystore and passwords:

```sh
keytool -genkeypair -keystore chordpad-release.jks -alias chordpad \
  -keyalg RSA -keysize 3072 -validity 10000
base64 < chordpad-release.jks | gh secret set ANDROID_KEYSTORE_BASE64 --repo dlip/chordpad
gh secret set ANDROID_KEYSTORE_PASSWORD --repo dlip/chordpad
gh secret set ANDROID_KEY_ALIAS --body chordpad --repo dlip/chordpad
gh secret set ANDROID_KEY_PASSWORD --repo dlip/chordpad
```

GitHub CLI prompts for the passwords. Keep the same signing key for future
releases so users can install updates. A local debug build uses a different key;
export your layout before uninstalling it to switch to the release APK, then
configure your mappings and accessibility service again after installation.

Keep pending changes under `## Unreleased`. Before a release, move them under a
heading for the version you are tagging, and commit that changelog entry with
the release's code. Preview the notes from the repository root:

```sh
python3 scripts/release_notes.py v1.0.1
```

After merging the changes and setting the secrets, publish the tag:

```sh
git tag v1.0.1
git push origin v1.0.1
```


To check held modifiers and pointer selection, unlock the tablet and run from
the repository root:

```sh
nix-shell android/shell.nix --run 'cd android && ./gradlew :app:assembleDebug :gamepad-check:assembleDebug'
adb install -r android/app/build/outputs/apk/debug/app-debug.apk
adb install -r android/gamepad-check/build/outputs/apk/debug/gamepad-check-debug.apk
./android/start-modifiers.sh
adb shell am instrument -w io.github.dlip.chordpad.gamepadcheck/io.github.dlip.chordpad.ModifierCheck
adb uninstall io.github.dlip.chordpad.gamepadcheck
```

This uses a virtual gamepad and mouse in a dedicated test editor. It checks the
real Android modifier state, Shift-click and Shift-tap range selection, and
release on controller disconnect. It restores any connected original controller and leaves
remapping off.

Pass `-e helper false` to `ModifierCheck` with the helper stopped to check
Shift-click and Shift-tap selection without global input privileges.

Use `-e web true -e pointer touchpad_click` or `-e web true -e pointer touchpad_tap`
to check physical-click and surface-tap event sequences in a browser editor.
The pointer events use the gamepad device ID, mouse source, and finger tool type
reported by the DualSense touchpad.
