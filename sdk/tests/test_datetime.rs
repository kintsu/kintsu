macro_rules! test_time_lib {
    ($feature: literal; mod $name: ident for $t: path) => {
        paste::paste! {
            #[cfg(feature = $feature)]
            pub mod $name {
                use kintsu_sdk::namespace;

                include!("shared.rs");


                #[derive(kintsu_sdk::Struct)]
                #[fields(version = 1)]
                pub struct [<CommentWith $name:camel>] {
                    pub id: i64,
                    pub comment: String,
                    pub created_at: $t,
                    pub deleted_at: Option<$t>,
                }

                namespace! {
                    "abc.corp.exts" { [<CommentWith $name:camel>] }
                }

                #[test]
                fn [<smoke_comment_with_ $name:snake>](){
                    smoke_basic::<[<CommentWith $name:camel>], _>(
                        concat!("../samples/test-struct-with-", $feature, ".toml"),
                        |def| {
                            kintsu_testing::assert_yaml_snapshot!{
                                def
                            }
                        }
                    );
                }
            }
        }
    };
}

test_time_lib! {
    "chrono"; mod test_chrono for chrono::DateTime::<chrono::Utc>
}

test_time_lib! {
    "time"; mod test_time for time::UtcDateTime
}
