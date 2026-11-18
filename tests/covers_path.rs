#[test]
fn real_titles_resolve() {
    for t in [
        "Crisis Core - Final Fantasy VII",
        "God of War - Ghost of Sparta",
        "Patapon 3",
        "Shin Megami Tensei - Persona 3 Portable",
    ] {
        assert!(
            t_emulator::covers::path_for(t).is_some(),
            "no cover for {t}"
        );
    }
}
