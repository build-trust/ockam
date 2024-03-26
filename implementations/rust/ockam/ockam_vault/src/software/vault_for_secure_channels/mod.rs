use cfg_if::cfg_if;

#[cfg(any(
    feature = "OCKAM_XX_25519_AES128_GCM_SHA256",
    feature = "OCKAM_XX_25519_AES256_GCM_SHA256",
    not(feature = "disable_default_noise_protocol")
))]
cfg_if! {
    // only linux-gnu amd64 and armv8 are supported
    // without recompiling the bindings
    if #[cfg(all(
        any(target_arch="x86_64", target_arch="aarch64"),
        target_os="linux",
        target_env="gnu"))] {
        mod aes_aws_lc;
        use aes_aws_lc::make_aes;
    } else {
        mod aes_rs;
        use aes_rs::make_aes;
    }
}

mod types;
#[allow(clippy::module_inception)]
mod vault_for_secure_channels;

pub use types::*;
pub use vault_for_secure_channels::*;
