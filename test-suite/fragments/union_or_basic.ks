namespace pkg;

namespace types {
	// Basic oneof type alias: TSY-0008
	type StringOrInt = oneof str | i32;

    // Multi-type oneof
	type JsonValue = oneof str | i32 | f64 | bool;

    // RFC-0016: Union-or with struct merge
	struct Success {
		message: str
	};

    struct Error {
		code: i32
	};

    // Union-or merges fields, conflicts become oneof
	struct Base {
		id: i64,
		name: str
	};

    struct Extra {
		email: str
	};

    type UserInfo = Base &| Extra;
};
