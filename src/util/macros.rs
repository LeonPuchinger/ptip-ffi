/// Creates a [`HashMap`] from the given key–value pairs.
///
/// `map!` allows `HashMap`s to be defined with a concise, literal-like syntax.
/// There are two main ways to use this macro:
///
/// - Create a `HashMap` with type inference:
///
/// ```
/// use std::collections::HashMap;
/// use ptip_ffi::map;
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
/// use ptip_ffi::map;
///
/// let m: HashMap<String, i32> = map! {
///     "a".to_string() => 1,
///     "b".to_string() => 3,
/// };
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

    // Untyped entries, type inferred from contents or
    // the type annotation of the assignee.
    ( $( $key:expr => $value:expr ),+ $(,)? ) => {{
        let mut m = ::std::collections::HashMap::new();
        $( m.insert($key, $value); )*
        m
    }};
}
