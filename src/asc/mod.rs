use crate::asc::asc_base::{AscPtr, AscType, AscValue};
use crate::asc::errors::AscError;
use crate::{impl_asc_type_enum, impl_asc_type_struct};
use std::fmt::Debug;
use std::marker::PhantomData;

pub mod asc_base;
pub mod asc_types;
pub mod errors;
pub mod macros;

struct TestAb<T> {
    name: AscPtr<u32>,
    data: PhantomData<T>,
}

impl_asc_type_struct!(TestAb<T>; name => AscPtr<u32>, data => PhantomData<T>);

struct TestAb2<T: Debug> {
    name: AscPtr<u32>,
    data: PhantomData<T>,
}

impl_asc_type_struct!(TestAb2<T: Debug>; name => AscPtr<u32>, data => PhantomData<T>);

struct TestAb3<T: Debug, K: Sync> {
    name: AscPtr<u32>,
    data: PhantomData<T>,
    b: PhantomData<K>,
}

impl_asc_type_struct!(TestAb3<T: Debug, K: Sync>; name => AscPtr<u32>, data => PhantomData<T>, b => PhantomData<K>);

struct TestAb4 {}

impl_asc_type_struct!(TestAb4;);

enum TestEnum1 {
    A,
    B,
}

impl_asc_type_enum!(TestEnum1; A => 0, B => 1);

enum TestEnum {
    A(u32),
    B(i32),
}

impl_asc_type_enum!(TestEnum; A(u32) => 0, B(i32) => 1);

enum TestEnum2<T> {
    A(T),
    B(i32),
}

impl_asc_type_enum!(TestEnum2<T>; A(T) => 0, B(i32) => 1);