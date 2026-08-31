//! Las cestas iniciales del SPEC existen como datos editables, no `match` del selector.

use batuta_quality::initial_action_profiles;

#[test]
fn las_cestas_iniciales_cubren_las_acciones_documentadas_y_suman_cien() {
    let profiles = initial_action_profiles(70, 2_592_000).unwrap();

    for action in [
        "implementation",
        "repair",
        "code_generation",
        "code_execution",
        "tools",
        "web_research",
        "review",
        "documentation",
    ] {
        let profile = &profiles[action];
        assert_eq!(
            profile
                .basket
                .iter()
                .map(|item| u16::from(item.weight))
                .sum::<u16>(),
            100
        );
    }
    assert!(
        profiles["implementation"]
            .basket
            .iter()
            .any(|item| item.benchmark == "swe-bench")
    );
    assert!(
        profiles["web_research"]
            .basket
            .iter()
            .any(|item| item.benchmark == "gaia")
    );
    assert!(
        profiles["tools"]
            .basket
            .iter()
            .any(|item| item.benchmark == "bfcl")
    );
}
