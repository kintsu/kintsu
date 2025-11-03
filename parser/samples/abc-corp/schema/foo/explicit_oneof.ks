namespace foo;

oneof ComplexOneOf {
	FormA(i32),
	FormB {
		desc: str
	}
};
