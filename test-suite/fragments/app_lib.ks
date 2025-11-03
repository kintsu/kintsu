namespace app;

namespace types {
	use lib_a;
	use lib_b;

	struct Combined {
		a: lib_a::events::EventA,
		b: lib_b::events::EventB
	};
};
