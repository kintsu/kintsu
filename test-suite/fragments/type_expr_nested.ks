namespace pkg;

namespace types {
    // Base struct
    struct Profile {
        username: str,
        bio?: str,
        avatar?: str,
        settings: {
            theme: str,
            notifications: bool
        }
    };

    type Settings = Profile::settings;

    // RFC-0018: Nested type expressions
    type ProfileCreateInput = Omit[Required[Profile], settings];

    // Type expression with oneof (not standalone union-or)
    type ConfigValue = oneof str | i32 | bool;

    // Complex nested struct
    struct Organization {
        id: i64,
        name: str,
        owner: Profile,
        members: Profile[]
    };

    // Pick from nested struct
    type OrgSummary = Pick[Organization, id | name];

    // Internal-tagged oneof with struct variants
    #[tag(name = "variant")]
    oneof DataPacket {
        Text { content: str, encoding?: str },
        Binary { data: str, compressed?: bool },
        Json { payload: str }
    };
};
