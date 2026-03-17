/// Creates a [`HashMap`] from the given key–value pairs.
///
/// `map!` allows `HashMap`s to be defined with a concise, literal-like syntax.
/// There are two main forms of this macro:
///
/// - Create a `HashMap` with type inference:
///
/// ```
/// use std::collections::HashMap;
///
/// let m = map! {
///     "a" => 1,
///     "b" => 3,
/// };
/// assert_eq!(m["a"], 1);
/// assert_eq!(m["b"], 3);
/// ```
///
/// - Create a `HashMap` with explicit key and value types:
///
/// ```
/// use std::collections::HashMap;
///
/// let m = map! {
///     String => i32,
///     "a".to_string() => 1,
///     "b".to_string() => 3,
/// };
/// let m2 = map! { String => i32, }; // empty, typed map
/// ```
///
/// Trailing commas after the last entry are allowed. For empty maps without
/// explicit types, the key and value types must be inferred from context.
///
/// [`HashMap`]: std::collections::HashMap
#[macro_export]
macro_rules! map {
    // Empty map
    () => {
        ::std::collections::HashMap::new()
    };

    // Typed, possibly empty: map! { K => V, }
    ($K:ty => $V:ty $(,)?) => {{
        ::std::collections::HashMap::<$K, $V>::new()
    }};

    // Untyped entries, type inferred from contents
    ( $( $key:expr => $value:expr ),+ $(,)? ) => {{
        let mut m = ::std::collections::HashMap::new();
        $( m.insert($key, $value); )*
        m
    }};

    // Typed entries: map! { K => V, "a" => 1, "b" => 3, }
    ( $K:ty => $V:ty, $( $key:expr => $value:expr ),+ $(,)? ) => {{
        let mut m = ::std::collections::HashMap::<$K, $V>::new();
        $( m.insert($key, $value); )*
        m
    }};
}