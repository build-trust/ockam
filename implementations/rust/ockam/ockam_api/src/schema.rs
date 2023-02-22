#[allow(dead_code)]
pub const SCHEMA: &str = core::include_str!("schema.cddl");

#[cfg(feature = "tag")]
pub mod tag {

    use {
        super::SCHEMA, cddl_cat::context::BasicContext, ockam_core::api::merged_cddl,
        once_cell::race::OnceBox,
    };

    pub fn cddl() -> &'static BasicContext {
        static INSTANCE: OnceBox<BasicContext> = OnceBox::new();
        INSTANCE.get_or_init(|| Box::new(merged_cddl(&[SCHEMA]).unwrap()))
    }
}
