namespace errors;

#![version(1)]
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
		desc: str
	}
};
