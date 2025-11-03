use kintsu_sdk::Definitions;

#[allow(unused)]
fn smoke_basic<D: kintsu_sdk::Defined, F: Fn(&'static Definitions)>(
    out: &'static str,
    snap: F,
) {
    use kintsu_sdk::Defined;

    kintsu_testing::insta_test!(|| {
        snap(D::definition());
    });

    let ser = toml::to_string(D::definition()).unwrap();
    std::fs::write(out, ser).unwrap();
}
