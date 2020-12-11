macro_rules! api_obj_impl {
    ($class:ident, $($rename:expr => $field:ident: $ty:ty),+) => {
        #[derive(Clone, Debug, Deserialize, Serialize)]
        pub struct $class {
            $(
                #[serde(rename = $rename)]
                pub $field: $ty
            ),+
        }

        display_impl!($class, $($field),+);
    };
}

macro_rules! display_impl {
    ($class:ident, $($field:ident),+) => {
        impl std::fmt::Display for $class {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, stringify!($class))?;
                write!(f, "{{")?;
                $(
                    write!(f, "{}: {:?}", stringify!(Self.$field), self.$field)?;
                )+
                write!(f, "}}")
            }
        }
    };
}
