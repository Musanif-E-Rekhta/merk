#[macro_export]
macro_rules! from_as_internal {
    ($($src:ty => $origin:literal),* $(,)?) => {
        $(impl ::std::convert::From<$src> for $crate::error::Error {
            fn from(e: $src) -> Self {
                $crate::error::Error::internal($origin, ::std::string::ToString::to_string(&e))
            }
        })*
    };
}

#[macro_export]
macro_rules! from_as_bad_request {
    ($($src:ty => $code:literal),* $(,)?) => {
        $(impl ::std::convert::From<$src> for $crate::error::Error {
            fn from(e: $src) -> Self {
                $crate::error::Error::bad_request($code, ::std::string::ToString::to_string(&e))
            }
        })*
    };
}
