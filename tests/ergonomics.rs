use chordpad::model::*;

#[test]
fn shoulders_use_index_alone_and_recruit_middle_for_the_trigger_when_paired() {
    for (bumper, trigger, index, middle) in [
        (
            Button::L1,
            Button::L2,
            Finger::LeftIndex,
            Finger::LeftMiddle,
        ),
        (
            Button::R1,
            Button::R2,
            Finger::RightIndex,
            Finger::RightMiddle,
        ),
    ] {
        let input = |mask| Input {
            dpad: Dpad::Neutral,
            mask,
        };
        let empty = Input::default();
        let first = input(bumper.bit());
        let second = input(trigger.bit());
        let both = input(bumper.bit() | trigger.bit());
        assert_eq!(first.fingers(empty), index.bit());
        assert_eq!(second.fingers(empty), index.bit());
        assert_eq!(both.fingers(empty), index.bit() | middle.bit());
        assert_eq!(first.fingers(second), index.bit());
        assert_eq!(second.fingers(first), middle.bit());
        assert_eq!(empty.fingers(second), 0);
        assert_eq!(both.middle_fingers(), middle.bit());
        assert_eq!(first.middle_fingers(), 0);
        assert_eq!(second.middle_fingers(), 0);
    }
    let thumbs = Input {
        dpad: Dpad::UpRight,
        mask: Button::A.bit() | Button::X.bit(),
    };
    assert_eq!(
        thumbs.fingers(Input::default()),
        Finger::LeftThumb.bit() | Finger::RightThumb.bit()
    );
    assert_eq!(thumbs.middle_fingers(), 0);
}

#[test]
fn diagonal_comfort_is_mirrored_and_applies_with_shoulders_and_held_context() {
    let mut costs = Costs::default();
    let dpad = |dpad| Input { dpad, mask: 0 };
    let face = |a: Button, b: Button| Input {
        dpad: Dpad::Neutral,
        mask: a.bit() | b.bit(),
    };
    for shoulder in [0, Button::L1.bit()] {
        for (easy, hard) in [
            (Dpad::DownRight, Dpad::UpRight),
            (Dpad::UpLeft, Dpad::DownLeft),
        ] {
            let easy = Input {
                mask: shoulder,
                ..dpad(easy)
            };
            let hard = Input {
                mask: shoulder,
                ..dpad(hard)
            };
            assert_eq!(costs.effort(easy, 0), costs.effort(hard, 0));
            costs.awkward_diagonal_extra = 0.6;
            assert!((costs.effort(hard, 0) - costs.effort(easy, 0) - 0.6).abs() < 1e-12);
            costs.awkward_diagonal_extra = 0.0;
        }
    }
    costs.awkward_diagonal_extra = 0.6;
    for shoulder in [0, Button::R1.bit(), Button::R2.bit()] {
        for (easy, hard) in [
            (face(Button::A, Button::X), face(Button::X, Button::Y)),
            (face(Button::Y, Button::B), face(Button::A, Button::B)),
        ] {
            let easy = Input {
                mask: easy.mask | shoulder,
                ..easy
            };
            let hard = Input {
                mask: hard.mask | shoulder,
                ..hard
            };
            assert!(
                (costs.effort(hard, Button::L2.bit()) - costs.effort(easy, Button::L2.bit()) - 0.6)
                    .abs()
                    < 1e-12
            );
        }
    }
    let input = Input {
        dpad: Dpad::Neutral,
        mask: Button::A.bit(),
    };
    let difference = costs.effort_with_context(input, dpad(Dpad::UpRight))
        - costs.effort_with_context(input, dpad(Dpad::DownRight));
    assert!((difference - 0.6 * costs.held_load).abs() < 1e-12);
}

#[test]
fn new_costs_are_optional_validated_and_survive_layout_roundtrips() {
    let mut legacy: serde_json::Value =
        serde_json::from_str(include_str!("../layout.json")).unwrap();
    for field in [
        "skipgram_weights",
        "awkward_diagonal_extra",
        "same_finger_transition",
        "grip_change",
    ] {
        legacy["profile"]["costs"]
            .as_object_mut()
            .unwrap()
            .remove(field);
    }
    let original: Layout = serde_json::from_value(legacy).unwrap();
    assert_eq!(original.profile.costs.skipgram_weights, [0.0; 3]);
    assert_eq!(original.profile.costs.awkward_diagonal_extra, 0.0);
    assert_eq!(original.profile.costs.same_finger_transition, 0.0);
    assert_eq!(original.profile.costs.grip_change, 0.0);
    let mut layout = original.clone();
    layout.profile.costs.skipgram_weights = [0.5, 0.25, 0.125];
    layout.profile.costs.awkward_diagonal_extra = 0.6;
    layout.profile.costs.same_finger_transition = 1.0;
    layout.profile.costs.grip_change = 0.7;
    layout.validate().unwrap();
    let decoded: Layout = serde_json::from_str(&serde_json::to_string(&layout).unwrap()).unwrap();
    assert_eq!(decoded.profile.costs, layout.profile.costs);
    assert_eq!(decoded.bindings, original.bindings);
    for value in [-1.0, f64::NAN, f64::INFINITY] {
        let mut profile = layout.profile.clone();
        profile.costs.awkward_diagonal_extra = value;
        assert!(profile.validate().is_err());
        let mut profile = layout.profile.clone();
        profile.costs.same_finger_transition = value;
        assert!(profile.validate().is_err());
        let mut profile = layout.profile.clone();
        profile.costs.grip_change = value;
        assert!(profile.validate().is_err());
        for index in 0..3 {
            let mut profile = layout.profile.clone();
            profile.costs.skipgram_weights[index] = value;
            assert!(profile.validate().is_err());
        }
    }
    let json = serde_json::to_value(&original).unwrap();
    assert!(json["profile"]["costs"].get("skipgram_weights").is_none());
    assert!(
        json["profile"]["costs"]
            .get("same_finger_transition")
            .is_none()
    );
    assert!(json["profile"]["costs"].get("grip_change").is_none());
    assert!(
        json["profile"]["costs"]
            .get("awkward_diagonal_extra")
            .is_none()
    );
}
