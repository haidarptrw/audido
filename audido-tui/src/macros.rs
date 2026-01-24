// =================================================================================
//  KEY HANDLER MACRO SYSTEM
// =================================================================================

/// Macro to define key handlers with dispatch logic.
///
/// Usage:
/// ```
/// handlers!(state, handle, key => {
///     fn name(KeyPattern, condition) { body }
///     ...
/// });
/// ```
///
/// The state, handle, and key identifiers are passed explicitly to work around
/// macro hygiene rules.
#[macro_export]
macro_rules! handlers {
    (
        $state:ident, $handle:ident, $key:ident => {
            $(
                fn $name:ident ( $pat:pat $(, $cond:expr)? ) $body:block
            )*
        }
    ) => {
        #[allow(unreachable_code)]
        #[allow(unused_variables)]
        match $key {
            $(
                $pat $(if $cond(&$state))? => {
                    $body
                    Ok(false)
                },
            )*
            _ => Ok(false),
        }
    }
}
