# Development

Run commands from the repository root with Rust and Cargo installed. On Nix,
enter `nix-shell` first.

```sh
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
```

See [PLAN.md](../PLAN.md) for chord recognition rules and the remaining milestones.

## Runtime validation

After the checks above, run the hardware-backed typing tests:

```sh
cargo test --locked --test typing -- --ignored --nocapture --test-threads=1
```

This command requires uinput and input-node permissions. It captures its own
virtual keyboard exclusively before emitting events, then checks a dry-run
typing subprocess using an exclusively grabbed virtual gamepad. These checks
cover real evdev/uinput events without typing into desktop applications.

Kernel-synthesized release reports and forced report splits cancel pending
strokes and refresh controller state. The input core emits these boundaries
during [device teardown and reset](https://github.com/torvalds/linux/blob/master/drivers/input/input.c);
they must not commit a typing chord.

The Kishi Ultra has passed a live inspector test for all six digital buttons,
four cardinal D-pad directions, full-range triggers, a three-button chord, and
held Shift/layer lookup. Its live recording replayed identically. Runtime dry-run
startup, signal pause/resume, and SIGTERM shutdown also passed. Editor/terminal
typing and Wayland/X11 session compatibility still need hands-on verification.

See the [Android guide](../android/README.md#build-and-checks) for Android build
requirements and checks.
