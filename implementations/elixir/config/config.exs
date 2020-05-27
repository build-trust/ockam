import Config

config :logger,
  backends: [:console],
  level: :info,
  handle_otp_reports: true,
  handle_sasl_reports: true

config :logger, :console,
  metadata: :all,
  colors: [enabled: true],
  format: "\n[$level]\nTimestamp: $date $time\n$message\n$metadata\n\n"

import_config "#{Mix.env()}.exs"
