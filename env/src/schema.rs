// @generated automatically by Diesel CLI.

diesel::table! {
    package (registry, name, version) {
        registry -> Text,
        name -> Text,
        version -> Text,
        content_hash -> Text,
        content -> Text,
    }
}

diesel::table! {
    registry (name) {
        name -> Text,
        url -> Text,
    }
}

diesel::joinable!(package -> registry (registry));

diesel::allow_tables_to_appear_in_same_query!(package, registry,);
