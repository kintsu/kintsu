#![allow(dead_code, unused_imports)]

pub mod operations;

#[test]
fn smoke_crate_private() {
    operations::abc_corp_test::BasicStruct {
        a: 1,
        b: Default::default(),
        c: vec![0.0],
        d: Default::default(),
    };
}
