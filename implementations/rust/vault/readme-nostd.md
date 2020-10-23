
# No_std Notes

This configuration of vault and the examples/vault-stm32f4-discovery is
setup to build a no_std ockam_vault library, with a minimal set of
execution of the vault secret_generate() and secret_export().

Incorporation of this branch as a PR is not recommended until further
cleanup and items in the notes are addressed.

To build, the no_std config required some late stage changes that are supported
only with the nightly config. If using rustup, the following will configure
the nightly toolchain, and add a target for the stm32f4 instruction set.

$ rustup install nightly
$ rustup target add thumbv7em-none-eabi

If the nightly toolchain is desired for global use

	$ rustup default nightly

If the nightly toolchain is only desired in the example directory

    $ cd <proj_root>/ockam/implementations/rust/examples/vault-stm32f4-discovery
    $ rustup override set nightly

Because of the .cargo/config in the example project the thumbv7em-none-eabi
should automatically be selected.

    $ cargo build

If building just the vault crate

    $ cargo build --features=nostd-stm32f4 --target=thumbv7em-none-eabi

Make sure either the nightly use is either set globally, or repeat the override
in the example directory.


## Notes, Loose Ends, and Bugs

* vault config is only partially feature selectable for nostd-stm32f4
  the configuration will not build without modification of Cargo.toml
  rand dependency configuration.

* the xeddsa also contains modifications that need a cleaner feature toggle
  between std and no_std modes

* due to above, the vault with std config builds, but is not tested atm.

* build of this configuration also requires nightly

* the nostd-stm32f4 feature could possibly refactored into separate
  nostd and stm32f4 features in which might be better suited to supporting
  multiple embedded backend implementations

* global_allocator was unexpectely required by the cargo build to be located
  in the ockam_vault instead of in the example project code. This noted in
  comments, but should be further examined.

* the ockam_common package was dropped from the vault as a time compromise
  ockam_common pulls in ockam_message as a dependency w/ numerous std:: constructs for tcp/ip support. The items used in vault from ockam_common only seemed to be a single macro. The macro was copied in, and the dependency
  dropped

* generally dependencies were toggled into no_std configurations, because there
  are currently several different approaches that libraries use to configure
  no_std, some do not allow automatic derivation of if std in in the current
  build, and cargo may have some design limitations around the transmission
  of no_std between levels of dependencies. This leaves the ockam_vault in
  a similar situation where more work may be required to discover or work in
  a upstream fix to provide a common method to control downstream no_std
  dependency environments as well as respond to upstream feature selection of
  std vs no_std.

* The stm32f407VG contains a hardware RNG function. If appropriate for
  cryptographic uses, it could be hooked up as the RNG used in the
  software/mod vault implementation. Currently a software rng with a fixed seed
  was selected for expediency. Without a hw rng, the seed could be generated
  from entropy within the device.

* Due to the workspace Cargo config, the cargo location of the example link
  script "memory.x" is searched for at the location of the root workspace Cargo.toml, not the example directory. This should be fixed.

* gradle build not integrated / tested
