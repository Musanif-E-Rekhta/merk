#[macro_export]
macro_rules! from_as_error {
    () => {};

    // internal/upstream with source preserved
    ($src:ty => internal($label:literal) + src, $($rest:tt)*) => {
        impl ::std::convert::From<$src> for $crate::error::Error {
            fn from(e: $src) -> Self {
                $crate::error::Error::Internal {
                    origin: ::std::borrow::Cow::Borrowed($label),
                    message: ::std::string::ToString::to_string(&e),
                    source: Some($crate::anyhow::Error::new(e)),
                }
            }
        }
        $crate::from_as_error!($($rest)*);
    };

    ($src:ty => upstream($label:literal) + src, $($rest:tt)*) => {
        impl ::std::convert::From<$src> for $crate::error::Error {
            fn from(e: $src) -> Self {
                $crate::error::Error::Upstream {
                    origin: ::std::borrow::Cow::Borrowed($label),
                    message: ::std::string::ToString::to_string(&e),
                    source: Some($crate::anyhow::Error::new(e)),
                }
            }
        }
        $crate::from_as_error!($($rest)*);
    };

    // all other variants via constructor method
    ($src:ty => $kind:ident($label:literal), $($rest:tt)*) => {
        impl ::std::convert::From<$src> for $crate::error::Error {
            fn from(e: $src) -> Self {
                $crate::error::Error::$kind($label, ::std::string::ToString::to_string(&e))
            }
        }
        $crate::from_as_error!($($rest)*);
    };
}
