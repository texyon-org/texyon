//! Enums for TeX's small numeric code sets: `chr_code` modifiers and the like.

/// Declares a fieldless enum with explicit discriminants plus `from_raw`
/// (a real `match`, so dispatch compiles to a jump table) and `raw`.
macro_rules! code_enum {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident: $repr:ty {
            $($(#[$vmeta:meta])* $var:ident = $val:expr),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Copy, Clone, PartialEq, Eq, Debug)]
        #[repr($repr)]
        $vis enum $name {
            $($(#[$vmeta])* $var = $val),+
        }

        #[allow(dead_code)]
        impl $name {
            #[inline]
            pub(crate) const fn from_raw(v: i32) -> Option<Self> {
                #![allow(non_upper_case_globals)]
                $(const $var: i32 = $val;)+
                match v {
                    $($var => Some(Self::$var),)+
                    _ => None,
                }
            }

            #[inline]
            pub(crate) const fn raw(self) -> i32 {
                self as i32
            }
        }
    };
}

pub(crate) use code_enum;
