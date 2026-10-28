use super::*;

#[test]
fn set_rev_ignores_array_lines_and_refuses_what_it_cannot_edit() {
    let text = "[harness]\nextra = [\n  [\"a\"],\n]\nsource = \"x\"\nrev = \"old\"\n";
    let out = set_rev(text, "abc", "v1").unwrap();
    assert!(out.ends_with("rev = \"abc\"  # v1\n"), "{out}");
    assert_eq!(out.matches("rev =").count(), 1);
    assert!(set_rev(
        "[harness]\nsource = \"x\"\n\"rev\" = \"old\"\n",
        "abc",
        "v1"
    )
    .is_err());
    assert!(set_rev("[harness]\nsource = \"x\"\n", "abc", "a\nb").is_err());
}

#[test]
fn only_a_full_hexadecimal_hash_is_fetched_by_hash() {
    assert!(is_full_hash(&"a1".repeat(20)));
    assert!(is_full_hash(&"A1".repeat(32)));
    assert!(!is_full_hash(&"a1".repeat(6)));
    assert!(!is_full_hash(&"g".repeat(40)));
    assert!(!is_full_hash("main"));
}

// spec: SYN-43 (internal: the format compiled in the binary is not printed)
#[test]
fn the_binary_reads_the_format_of_its_own_revision() {
    let declared =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../cardinal-format"))
            .unwrap();
    assert_eq!(declared.trim().parse::<u32>().unwrap(), FORMAT);
}
