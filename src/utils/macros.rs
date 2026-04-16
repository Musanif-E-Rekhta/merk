#[macro_export]
macro_rules! from_as_error {
    ($($src:ty => $kind:ident($label:literal)),* $(,)?) => {
        $(impl ::std::convert::From<$src> for $crate::error::Error {
            fn from(e: $src) -> Self {
                $crate::error::Error::$kind($label, ::std::string::ToString::to_string(&e))
            }
        })*
    };
}
