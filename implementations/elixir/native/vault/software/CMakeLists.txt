
add_library(ockam_elixir_vault_software)
add_library(ockam::elixir_vault_software ALIAS ockam_elixir_vault_software)

target_sources(
  ockam_elixir_vault_software
  PRIVATE
    nifs.c
)

target_include_directories(ockam_elixir_vault_software PUBLIC $ENV{ERL_INCLUDE_DIR})
set_target_properties(ockam_elixir_vault_software PROPERTIES COMPILE_FLAGS "-fPIC")
set_target_properties(ockam_elixir_vault_software PROPERTIES LINK_FLAGS "-dynamiclib -undefined dynamic_lookup")
