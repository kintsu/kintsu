namespace pkg;

namespace items {
	struct Foo {
		value: str
	};

	struct Bar {
		count: i32
	};

	type Result = oneof Foo | Bar | str;
};
