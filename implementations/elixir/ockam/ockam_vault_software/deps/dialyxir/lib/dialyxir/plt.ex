# Credits: this code was originally part of the `dialyze` task
# Copyright by James Fish
# https://github.com/fishcakez/dialyze

defmodule Dialyxir.Plt do
  @moduledoc false

  import Dialyxir.Output
  alias Dialyxir.Formatter

  def check(plts, fun \\ &check_plt/4) do
    find_plts(plts, [], fun)
  end

  defp find_plts([{plt, apps} | plts], acc, fun) do
    case plt_files(plt) do
      nil ->
        find_plts(plts, [{plt, apps, nil} | acc], fun)

      beams ->
        apps_rest = Enum.flat_map(plts, fn {_plt2, apps2} -> apps2 end)
        apps = Enum.uniq(apps ++ apps_rest)
        check_plts([{plt, apps, beams} | acc], fun)
    end
  end

  defp find_plts([], acc, fun) do
    check_plts(acc, fun)
  end

  defp check_plts(plts, fun) do
    _ =
      Enum.reduce(plts, {nil, MapSet.new(), %{}}, fn {plt, apps, beams}, acc ->
        fun.(plt, apps, beams, acc)
      end)
  end

  defp check_plt(plt, apps, old_beams, {prev_plt, prev_beams, prev_cache}) do
    info("Finding applications for #{Path.basename(plt)}")
    cache = resolve_apps(apps, prev_cache)
    mods = cache_mod_diff(cache, prev_cache)
    info("Finding modules for #{Path.basename(plt)}")
    beams = resolve_modules(mods, prev_beams)
    check_beams(plt, beams, old_beams, prev_plt)
    {plt, beams, cache}
  end

  defp cache_mod_diff(new, old) do
    Enum.flat_map(new, fn {app, {mods, _deps}} ->
      case Map.has_key?(old, app) do
        true -> []
        false -> mods
      end
    end)
  end

  defp resolve_apps(apps, cache) do
    apps
    |> Enum.uniq()
    |> Enum.filter(&(not Map.has_key?(cache, &1)))
    |> Enum.map(&app_info/1)
    |> Enum.into(cache)
  end

  defp app_info(app) do
    app_file = Atom.to_charlist(app) ++ '.app'

    case :code.where_is_file(app_file) do
      :non_existing ->
        error("Unknown application #{inspect(app)}")
        {app, {[], []}}

      app_file ->
        Path.expand(app_file)
        |> read_app_info(app)
    end
  end

  defp read_app_info(app_file, app) do
    case :file.consult(app_file) do
      {:ok, [{:application, ^app, info}]} ->
        parse_app_info(info, app)

      {:error, reason} ->
        Mix.raise("Could not read #{app_file}: #{:file.format_error(reason)}")
    end
  end

  defp parse_app_info(info, app) do
    mods = Keyword.get(info, :modules, [])
    apps = Keyword.get(info, :applications, [])
    inc_apps = Keyword.get(info, :included_applications, [])
    runtime_deps = get_runtime_deps(info)
    {app, {mods, runtime_deps ++ inc_apps ++ apps}}
  end

  defp get_runtime_deps(info) do
    Keyword.get(info, :runtime_dependencies, [])
    |> Enum.map(&parse_runtime_dep/1)
  end

  defp parse_runtime_dep(runtime_dep) do
    runtime_dep = IO.chardata_to_string(runtime_dep)
    regex = ~r/^(.+)\-\d+(?|\.\d+)*$/
    [app] = Regex.run(regex, runtime_dep, capture: :all_but_first)
    String.to_atom(app)
  end

  defp resolve_modules(modules, beams) do
    Enum.reduce(modules, beams, &resolve_module/2)
  end

  defp resolve_module(module, beams) do
    beam = Atom.to_charlist(module) ++ '.beam'

    case :code.where_is_file(beam) do
      path when is_list(path) ->
        path = Path.expand(path)
        MapSet.put(beams, path)

      :non_existing ->
        error("Unknown module #{inspect(module)}")
        beams
    end
  end

  defp check_beams(plt, beams, nil, prev_plt) do
    plt_ensure(plt, prev_plt)

    case plt_files(plt) do
      nil ->
        Mix.raise("Could not open #{plt}: #{:file.format_error(:enoent)}")

      old_beams ->
        check_beams(plt, beams, old_beams)
    end
  end

  defp check_beams(plt, beams, old_beams, _prev_plt) do
    check_beams(plt, beams, old_beams)
  end

  defp check_beams(plt, beams, old_beams) do
    remove = MapSet.difference(old_beams, beams)
    plt_remove(plt, remove)
    check = MapSet.intersection(beams, old_beams)
    plt_check(plt, check)
    add = MapSet.difference(beams, old_beams)
    plt_add(plt, add)
  end

  defp plt_ensure(plt, nil), do: plt_new(plt)
  defp plt_ensure(plt, prev_plt), do: plt_copy(prev_plt, plt)

  defp plt_new(plt) do
    info("Creating #{Path.basename(plt)}")
    plt = erl_path(plt)
    _ = plt_run(analysis_type: :plt_build, output_plt: plt, apps: [:erts])
    :ok
  end

  defp plt_copy(plt, new_plt) do
    info("Copying #{Path.basename(plt)} to #{Path.basename(new_plt)}")

    new_plt
    |> Path.dirname()
    |> File.mkdir_p!()

    File.cp!(plt, new_plt)
  end

  defp plt_add(plt, files) do
    case MapSet.size(files) do
      0 ->
        :ok

      n ->
        Mix.shell().info("Adding #{n} modules to #{Path.basename(plt)}")
        plt = erl_path(plt)
        files = erl_files(files)

        {duration_us, _} =
          :timer.tc(fn -> plt_run(analysis_type: :plt_add, init_plt: plt, files: files) end)

        Mix.shell().info(Formatter.formatted_time(duration_us))
        :ok
    end
  end

  defp plt_remove(plt, files) do
    case MapSet.size(files) do
      0 ->
        :ok

      n ->
        info("Removing #{n} modules from #{Path.basename(plt)}")
        plt = erl_path(plt)
        files = erl_files(files)
        _ = plt_run(analysis_type: :plt_remove, init_plt: plt, files: files)
        :ok
    end
  end

  defp plt_check(plt, files) do
    case MapSet.size(files) do
      0 ->
        :ok

      n ->
        Mix.shell().info("Checking #{n} modules in #{Path.basename(plt)}")
        plt = erl_path(plt)
        _ = plt_run(analysis_type: :plt_check, init_plt: plt)
        :ok
    end
  end

  defp plt_run(opts) do
    try do
      :dialyzer.run([check_plt: false] ++ opts)
    catch
      {:dialyzer_error, msg} ->
        error(color(":dialyzer.run error: #{msg}", :red))
    end
  end

  defp plt_info(plt) do
    erl_path(plt)
    |> :dialyzer.plt_info()
  end

  defp erl_files(files) do
    Enum.reduce(files, [], &[erl_path(&1) | &2])
  end

  defp erl_path(path) do
    encoding = :file.native_name_encoding()
    :unicode.characters_to_list(path, encoding)
  end

  defp plt_files(plt) do
    info("Looking up modules in #{Path.basename(plt)}")

    case plt_info(plt) do
      {:ok, info} ->
        Keyword.fetch!(info, :files)
        |> Enum.reduce(MapSet.new(), &MapSet.put(&2, Path.expand(&1)))

      {:error, :no_such_file} ->
        nil

      {:error, reason} ->
        Mix.raise("Could not open #{plt}: #{:file.format_error(reason)}")
    end
  end
end
