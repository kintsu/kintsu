#[macro_export]
macro_rules! default {
    ($name: ident: $ty: path = $value: expr) => {
        paste::paste!{
            #[allow(dead_code)]
            pub(crate) fn [<default_ $name:snake>]() -> $ty {
                $value
            }
        }
    };
    ($name: ident: $ty: path = $value: expr) => {
        paste::paste!{
            #[allow(dead_code)]
            pub(crate) fn [<default_ $name:snake>]() -> $ty {
                $value
            }
        }
    };
    ( $($ty: path: { $($name: ident = $value: expr), + $(,)?}), + $(,)?) => {
        $($(
            $crate::default!{
                $name: $ty = $value
            }
        )*)*
    };
}

default! {
    bool: { yes = true, no = false }
}

#[macro_export]
macro_rules! map {
    ({
        $($field: ident: $value: expr), + $(,)?
    }) => {
        {
            let mut m = std::collections::BTreeMap::new();
            $(
                m.insert(stringify!($field).into(), $value.into());
            )*
            m
        }

    };
}

#[macro_export]
macro_rules! trace_replace {
    (
        $before: ident; $after: expr
    ) => {{
        let new = $after;
        if &new != $before {
            tracing::info!("type simplification: {} -> {}", $before, new);
        }
        new
    }};
}
