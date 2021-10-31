#![deny(clippy::all)]
#![allow(clippy::nonstandard_macro_braces)]

use napi::{CallContext, JsObject, JsString, Result};
use napi_derive::*;

#[module_exports]
fn init(mut exports: JsObject) -> Result<()> {
  exports.create_named_method("hello", hello)?;
  Ok(())
}

#[js_function(1)]
fn hello(ctx: CallContext) -> Result<JsString> {
  let argument_one = ctx.get::<JsString>(0)?.into_utf8()?;
  ctx.env.create_string_from_std(format!("Hello {}!", argument_one.as_str()?))
}
