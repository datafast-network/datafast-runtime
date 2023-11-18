#[macro_export]
macro_rules! schema {
    ($($k:ident => $v:expr),* $(,)?) => {{
        use std::iter::{Iterator, IntoIterator};
        use $crate::components::manifest_loader::schema_lookup::FieldKind;
        Iterator::collect(IntoIterator::into_iter([$((stringify!($k).to_string(), FieldKind{
            kind: $v,
            relation: None,
            list_inner_kind: None,
        }),)*]))
    }};
}

#[macro_export]
macro_rules! entity {
    ($($k:ident => $v:expr),* $(,)?) => {{
        use std::iter::{Iterator, IntoIterator};
        Iterator::collect(IntoIterator::into_iter([$((stringify!($k).to_string(), $v),)*]))
    }};
}
