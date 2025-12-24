pub use kintsu_core::{
    CompoundType, DeclDefined, Defined, Definitions, Enum, ErrorTy as Error, Field, FieldsList,
    Meta, Named, OneOf, OneOfVariant, Operation, StrOrInt, Struct, Type, Typed, VariantKind,
    Version, map, namespace, namespace::OfNamespace,
};

pub mod declare {
    //! Re-exports of parser declaration types.
    pub use kintsu_core::declare::*;
}

pub use kintsu_derives::{Enum, Error, OneOf, Struct, module, operation};
pub use serde_repr::{Deserialize_repr as IntDeserialize, Serialize_repr as IntSerialize};
