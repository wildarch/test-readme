use test_readme::test_install_md;

#[test]
fn install_md() {
    let flags = [("apt-get".into(), "-y".into())].iter().cloned().collect();
    test_install_md("debian:buster", flags, "tests/install.md")
        .expect("Failed to build image from instructions");
}
