//! Golden snapshot for the default theme. The bytes here are the contract
//! described in `docs/design-docs/default-theme.md`; any intentional change
//! requires updating both the snapshot and the design doc in one commit.

use ccstatusline_rs::render_string;

const FIXTURE: &str = include_str!("fixtures/default-payload.json");

#[test]
fn default_theme_matches_golden_string() {
    let out = render_string(FIXTURE).expect("render succeeds");
    let expected = concat!(
        "✦ [Opus 4.7 (1M context)] | 📂 F:\\Works\\naya\\cc-statusline-rust",
        " | 🔋 [..........] 80.0K/1.0M(8%) | 📊 85.3K | 💰 $2.55\n",
        "⏱ 5h [##........](21%) ↻ 12:00 | 📅 7d [##........](20%) ↻ 5/19 06:00",
    );
    assert_eq!(out, expected);
}

#[test]
fn default_theme_snapshot() {
    let out = render_string(FIXTURE).expect("render succeeds");
    insta::assert_snapshot!(out);
}
