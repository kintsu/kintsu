namespace pkg;

namespace types {
    // Base struct for type expressions
    struct User {
        id: i64,
        name: str,
        email: str,
        password_hash?: str,
        created_at: i64,
        updated_at?: i64
    };

    // RFC-0018: Pick operator - select specific fields
    type UserSummary = Pick[User, id | name];

    // RFC-0018: Omit operator - exclude specific fields
    type PublicUser = Omit[User, password_hash];

    // RFC-0018: Partial operator - make all fields optional
    type UserPatch = Partial[User];

    // RFC-0018: Partial with selectors
    type UserOptionalDates = Partial[User, created_at | updated_at];

    // RFC-0018: Required operator - make all fields required
    type StrictUser = Required[User];

    // Base oneof for type expressions
    oneof Status {
        Pending { since: i64 },
        Active { activated_at: i64 },
        Suspended { reason: str },
        Deleted { deleted_at: i64 }
    };

    // RFC-0018: Exclude operator - remove variants
    type ActiveStatus = Exclude[Status, Pending | Deleted];

    // RFC-0018: Extract operator - keep only specified variants
    type TerminalStatus = Extract[Status, Suspended | Deleted];

    // Array type
    type UserList = User[];

    // RFC-0018: ArrayItem operator
    type SingleUser = ArrayItem[UserList];
};
