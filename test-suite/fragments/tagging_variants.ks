namespace pkg;

namespace types {
    // External tagging (default)
    oneof Message {
        Text { content: str },
        Data { payload: str }
    };

    // Internal tagging
    #[tag(name = "type")]
    oneof Event {
        UserJoined { user_id: i64 },
        UserLeft { user_id: i64 }
    };

    // Adjacent tagging
    #[tag(name = "kind", content = "payload")]
    oneof ApiResponse {
        Success { data: str },
        Error { code: i32 }
    };
};
