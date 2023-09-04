import Config

aws_vault =
  case System.fetch_env("OCKAM_VAULT_AWS") do
    {:ok, "true"} -> true
    :error -> false
  end

config :ockly, aws_vault: aws_vault
