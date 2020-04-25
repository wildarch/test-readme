use test_readme::{build_markdown, Options};

#[test]
fn install_md() {
    build_markdown(
        "debian:buster",
        Options::default().flag("apt-get", "-y"),
        "tests/install.md",
    )
    .expect("Failed to build image from instructions");
}
