#[version(1)]
namespace test;

oneof ComplexOneOf {
	FormA(i32),
	FormB {
		desc: str
	}
};
