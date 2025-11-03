namespace test;

type UnionA = oneof str | i32;

#[version(1)]
struct MsgWithOneOfAlias {
	a: UnionA
};
