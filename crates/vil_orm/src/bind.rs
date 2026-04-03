//! VilBind — type-erased bind values for VilEntity query methods.
//!
//! Allows mixing &str, i64, f64, Option<String> in a single bind slice:
//! ```ignore
//! Profile::update_where(pool, "xp = xp + ?", "id = ?", vil_args![25_i64, "user-id"]).await?;
//! ```

use sqlx::any::AnyArguments;
use sqlx::Arguments;

/// Trait for values that can be bound to VilEntity queries.
pub trait VilBind: Send + Sync {
    fn bind_to(&self, args: &mut AnyArguments<'_>);
}

impl VilBind for &str {
    fn bind_to(&self, args: &mut AnyArguments<'_>) {
        let _ = args.add(self.to_string());
    }
}

impl VilBind for String {
    fn bind_to(&self, args: &mut AnyArguments<'_>) {
        let _ = args.add(self.clone());
    }
}

impl VilBind for i64 {
    fn bind_to(&self, args: &mut AnyArguments<'_>) {
        let _ = args.add(*self);
    }
}

impl VilBind for i32 {
    fn bind_to(&self, args: &mut AnyArguments<'_>) {
        let _ = args.add(*self as i64);
    }
}

impl VilBind for f64 {
    fn bind_to(&self, args: &mut AnyArguments<'_>) {
        let _ = args.add(*self);
    }
}

impl VilBind for bool {
    fn bind_to(&self, args: &mut AnyArguments<'_>) {
        let _ = args.add(*self as i32);
    }
}

// ── Option wrappers — encode NULL natively ──

/// Wraps Option<T> for NULL-safe binding in VilQuery.
/// Use via `vil_opt!(value)` macro or construct directly.
pub struct VilOptStr(pub Option<String>);
pub struct VilOptI64(pub Option<i64>);
pub struct VilOptF64(pub Option<f64>);

impl VilBind for VilOptStr {
    fn bind_to(&self, args: &mut AnyArguments<'_>) {
        let _ = args.add(self.0.clone());
    }
}

impl VilBind for VilOptI64 {
    fn bind_to(&self, args: &mut AnyArguments<'_>) {
        let _ = args.add(self.0);
    }
}

impl VilBind for VilOptF64 {
    fn bind_to(&self, args: &mut AnyArguments<'_>) {
        let _ = args.add(self.0);
    }
}

/// Build AnyArguments from a slice of VilBind values.
pub fn build_args<'q>(binds: &[&dyn VilBind]) -> AnyArguments<'q> {
    let mut args = AnyArguments::default();
    for b in binds {
        b.bind_to(&mut args);
    }
    args
}

/// Convenience macro for building bind args with mixed types.
///
/// ```ignore
/// use vil_orm::vil_args;
/// Profile::update_where_v(pool, "xp = xp + ?", "id = ?", vil_args![25_i64, "user-id"]).await?;
/// ```
#[macro_export]
macro_rules! vil_args {
    ($($val:expr),* $(,)?) => {{
        let args: Vec<&dyn $crate::bind::VilBind> = vec![$(&$val as &dyn $crate::bind::VilBind),*];
        args
    }};
}
