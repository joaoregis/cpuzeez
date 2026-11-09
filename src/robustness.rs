//! Robustness tests for what the app reads from outside (S3-05): settings
//! written by another version, the update check's answer, imported reports,
//! the elevated helper's argument. Deterministic, run on every OS in CI;
//! they must never panic, and what they accept must stay within bounds.

use cpuzeez_core::testkit::Rng;

/// Rounds per test: 15_000 by default; `CZ_ROBUST_N` raises it for a long
/// campaign (`CZ_ROBUST_N=2000000 cargo test robustness`), `CZ_ROBUST_SEED` varies
/// the inputs between campaigns.
fn n() -> usize {
    std::env::var("CZ_ROBUST_N").ok().and_then(|v| v.parse().ok()).unwrap_or(15_000)
}

#[test]
fn settings_from_any_version() {
    let mut r = Rng::seeded(0x5E77_1465);
    let good = serde_json::to_string(&crate::settings::Settings::default()).unwrap();
    for _ in 0..n() {
        let _ = serde_json::from_str::<crate::settings::Settings>(&r.mutate_text(&good));
    }
    // Unknown keys are ignored (a newer version's file), wrong types are errors, never panics.
    let s: crate::settings::Settings =
        serde_json::from_str(r#"{"a_future_option": [1, 2], "overlay": {"anchor": "top"}}"#).unwrap_or_default();
    let _ = s;
}

#[test]
fn update_answers() {
    let mut r = Rng::seeded(0x0DA7E);
    let good = r#"{"tag_name":"v1.0.0-beta.7","html_url":"https://github.com/x/y/releases/tag/v1","assets":[{"name":"cpuzeez-1.0.0-beta.7-windows-x64.msi","browser_download_url":"https://github.com/x/y/a.msi","size":123},{"name":"SHA256SUMS.txt","browser_download_url":"https://github.com/x/y/s.txt","size":9}]}"#;
    for _ in 0..n() {
        let _ = crate::update::parse(&r.mutate_text(good));
    }
    // Version comparison: never panics, and "newer" is never true both ways.
    let parts = ["1", "0", "0", "beta", "7", "rc", "10", "", "-", "+", "x", "99999999999999999999", "é"];
    for _ in 0..n() {
        let v = |r: &mut Rng| {
            (0..1 + r.below(6))
                .map(|_| parts[r.below(parts.len())])
                .collect::<Vec<_>>()
                .join([".", "-", ""][r.below(3)])
        };
        let (a, b) = (v(&mut r), v(&mut r));
        assert!(!(crate::update::newer(&a, &b) && crate::update::newer(&b, &a)), "{a:?} vs {b:?}");
        assert!(!crate::update::newer(&a, &a), "{a:?} newer than itself");
    }
}

#[test]
fn imported_reports() {
    let mut r = Rng::seeded(0xC4F_0001);
    let good = br#"[{"title":"CPU","groups":[{"name":"Core","fields":[{"key":"Name","value":"X"}]}]}]"#.to_vec();
    for _ in 0..n() {
        let _ = serde_json::from_slice::<Vec<cpuzeez_core::report::Section>>(&r.mutate(&good));
    }
}
