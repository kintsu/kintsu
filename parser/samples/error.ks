/* My Namespace */
#[version(1)]
namespace test;

enum ErrorCode {
	InternalError
};

struct ErrorDesc {
    desc: str,
    code: ErrorCode
};

error ServerError {
    Known(ErrorDesc),
    Unknown {
        /*
            nesting level
            + 1 is format
        */
        desc: str
    }
};
