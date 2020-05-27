defmodule TestCluster do
  # Defines a local multi-node distributed erlang test cluster.
  #
  # Adapted from https://github.com/whitfin/local-cluster
  # which is licenced under the MIT license.

  # Start the test cluster.
  @spec start :: :ok
  def start do
    :ok = :net_kernel.monitor_nodes(true)
    System.cmd("epmd", ["-daemon"])
    Node.start(:manager@localhost, :shortnames)
  end

  # Stops the test cluster.
  #
  # Turns the current node into a non-distributed erlang node. For the other
  # nodes in the cluster this looks like the `manager` node going down.
  @spec stop :: :ok | {:error, term}
  def stop, do: :net_kernel.stop()

  # Destroy all the
  @spec destroy_nodes([atom]) :: :ok
  def destroy_nodes(nodes) when is_list(nodes), do: Enum.each(nodes, &:slave.stop/1)

  @spec create_nodes(binary, integer, Keyword.t()) :: [atom]
  def create_nodes(prefix, count, options \\ [])
      when is_binary(prefix) and is_integer(count) and is_list(options) do
    start_nodes(prefix, count)
    |> setup_code_path
    |> setup_logging
    |> setup_mix
    |> load_applications(options)
    |> require_files(options)
  end

  defp start_nodes(prefix, count) do
    Enum.map(1..count, fn i ->
      {:ok, name} = :slave.start_link(:localhost, '#{prefix}#{i}')
      name
    end)
  end

  defp setup_code_path(nodes) do
    :rpc.multicall(nodes, :code, :add_paths, [:code.get_path()])
    nodes
  end

  defp setup_logging(nodes) do
    :rpc.multicall(nodes, Application, :ensure_all_started, [:logger])
    logger_config = Application.get_all_env(:logger)
    :rpc.multicall(nodes, Logger, :configure, [logger_config])

    nodes
  end

  defp setup_mix(nodes) do
    :rpc.multicall(nodes, Application, :ensure_all_started, [:mix])
    :rpc.multicall(nodes, Mix, :env, [Mix.env()])
    nodes
  end

  defp load_applications(nodes, options) do
    # copy all application environment values
    loaded_apps =
      for {app_name, _, _} <- Application.loaded_applications() do
        for {key, val} <- Application.get_all_env(app_name) do
          :rpc.multicall(nodes, Application, :put_env, [app_name, key, val])
        end

        app_name
      end

    # start applications in the specified order
    for app_name <- Keyword.get(options, :applications, loaded_apps), app_name in loaded_apps do
      :rpc.multicall(nodes, Application, :ensure_all_started, [app_name])
    end

    nodes
  end

  defp require_files(nodes, options) do
    for file <- Keyword.get(options, :files, []) do
      :rpc.multicall(nodes, Code, :require_file, [file])
    end

    nodes
  end
end
