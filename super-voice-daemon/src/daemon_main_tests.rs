// SPDX-License-Identifier: GPL-3.0-only
//! What the packaged systemd unit promises about starting and stopping.

fn unit() -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("systemd/super-voice.service");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The values of every `key=` line, comments skipped.
fn values<'a>(unit: &'a str, key: &str) -> Vec<&'a str> {
    unit.lines()
        .map(str::trim)
        .filter(|line| !line.starts_with('#'))
        .filter_map(|line| line.strip_prefix(key)?.strip_prefix('='))
        .collect()
}

/// The unit must never be what keeps `graphical-session.target` up. That
/// target stops itself once no unit needs it, and every app and portal that is
/// `PartOf=` it stops with it. With `Requires=` on it, a desktop whose own
/// units no longer held the target had every daemon stop take its apps down.
#[test]
fn the_unit_never_holds_up_the_session_target() {
    let unit = unit();
    for key in ["Requires", "Wants", "BindsTo", "Requisite", "Upholds"] {
        for value in values(&unit, key) {
            assert!(
                !value.contains("graphical-session.target"),
                "{key}={value} makes the daemon hold the desktop session up"
            );
        }
    }
    assert!(
        values(&unit, "PartOf")
            .iter()
            .any(|v| v.contains("graphical-session.target")),
        "the daemon should stop with the desktop session"
    );
    let wanted_by = values(&unit, "WantedBy").join(" ");
    for target in ["default.target", "graphical-session.target"] {
        assert!(
            wanted_by.split_whitespace().any(|t| t == target),
            "WantedBy= should name {target}: {wanted_by}"
        );
    }
}
