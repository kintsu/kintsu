use kintsu_sdk::Defined;
use test_gen_a::operations::abc_corp_test::*;

macro_rules! round_trip {
    ($expect: literal: $ty: path = $input: literal) => {
        paste::paste! {
            #[test]
            fn [<test_ $ty:snake>](){
                const EXPECT: &'static str = include_str!($expect);
                let def = $ty::definition();
                let as_toml = toml::to_string(def).unwrap().replace("\r", "");
                kintsu_testing::insta_test!(|| {
                    kintsu_testing::assert_yaml_snapshot!(def);
                });
                assert_eq!(EXPECT.replace("\r", ""), as_toml)
            }

            #[test]
            fn [<test_ $ty:snake _ser>](){
                const FROM: &'static str = include_str!($input);
                let de: $ty = serde_json::from_str(FROM.clone()).unwrap();

                kintsu_testing::insta_test!(|| {
                    kintsu_testing::assert_yaml_snapshot!(de);
                });
                assert_eq!(serde_json::from_str::<serde_json::Value>(FROM.clone()).unwrap(), serde_json::to_value(&de).unwrap())
            }
        }
    };
    ($($expect: literal: $ty: path = $input: literal), + $(,)?) => {
        $(
            round_trip!($expect: $ty = $input);
        )*
    };
}

round_trip! {
    "../../../samples/test-struct-readme.toml": BasicStructWithReadme = "../../../samples/test-struct-readme.json",
    "../../../samples/test-str-enum.toml": BasicStrEnum = "../../../samples/test-str-enum.json",
    "../../../samples/test-enum.toml": BasicIntEnum = "../../../samples/test-enum.json",
    "../../../samples/test-struct-with-enum.toml": SomeStructWithEnum = "../../../samples/test-struct-with-enum.json",
    "../../../samples/test-operation-error.toml": OperationError = "../../../samples/test-operation-error.json",
}
