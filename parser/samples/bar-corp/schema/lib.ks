namespace bar;

namespace baz {
	#![version(1)]
	enum Baz {
		Foo = 1,
		Bar = 2
	};

    type BazOrString = oneof Baz | str;
};
