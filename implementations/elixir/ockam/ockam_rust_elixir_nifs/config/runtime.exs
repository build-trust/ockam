import Config

aws_vault =
  case System.fetch_env("OCKAM_VAULT_AWS") do
    {:ok, "true"} -> true
    :error -> false
  end

config :ockam_rust_elixir_nifs, aws_vault: aws_vault
