defmodule Ockam.Identity.TrustPolicy do
  @moduledoc """
  Functions to check trust between identities.
  """

  alias Ockam.Identity

  @type identity_info() :: %{id: binary(), identity: Identity.t()}
  @type trust_rule() ::
          (function :: atom())
          | (function :: fun(2))
          | {function :: atom(), extra_args :: list()}
          | {function :: function(), extra_args :: list()}
          | {module :: atom(), function :: atom(), extra_args :: list()}

  @doc """
  Check multiple trust rules
  Rules can be defined as:

  - `function :: atom` - function (arity 2) from this module to run with my_info and contact_info
  - `{function :: atom, extra_args :: list}` - function from this module to run with my_info, contact_info and extra args
  - `{module :: atom, function :: atom, extra_args :: list}` - function from `module` to run with my_info, contact_info and extra args
  - `function :: function` - function (arity 2) to run with my_info and contact_info
  - `{function :: function, extra_args :: list}` - function to run with my_info, contact_info and extra args

  Each rule function should return `:ok | {:error, reason}`
  """
  @spec from_config([trust_rule()], my_info :: identity_info(), contact_info :: identity_info()) ::
          :ok | {:error, reason :: any()}
  def from_config(config, my_info, contact_info) do
    config = expand_config(config)

    Enum.reduce_while(config, :ok, fn rule, _acc ->
      case apply_rule(rule, my_info, contact_info) do
        :ok -> {:cont, :ok}
        {:error, reason} -> {:halt, {:error, reason}}
      end
    end)
  end

  @doc """
  Check contact identity using known identities storage via `Ockam.Identity.TrustPolicy.KnownIdentities` module

  If the contact is not present in known identities - refuse
  If the contact with the same ID exists in known identities - check the idenity history
    If history is equal - contact is trusted
    If history is newer - update the known contact
    If history is older of in conflict - refuse to trust the contact
  """
  def known_identity(
        _my_info,
        %{id: contact_id, identity: contact},
        known_identities_mod,
        extra_args \\ []
      ) do
    case known_identities_mod.get_identity(contact_id, extra_args) do
      {:ok, known_contact} ->
        case Identity.compare_identity_change_history(contact, known_contact) do
          {:ok, :equal} ->
            :ok

          {:ok, :newer} ->
            ## TODO: do we want to update the contact if it's changed?
            known_identities_mod.set_identity(contact_id, contact, extra_args)

          {:ok, :conflict} ->
            {:error,
             {:trust_policy, :known_identity, {:identity_conflict, contact, known_contact}}}

          {:ok, :older} ->
            {:error, {:trust_policy, :known_identity, {:identity_is_old, contact, known_contact}}}

          {:error, err} ->
            {:error, {:trust_policy, :known_identity, {:api_error, err}}}
        end

      {:error, :not_found} ->
        {:error, {:trust_policy, :known_identity, :unknown_identity}}

      {:error, reason} ->
        {:error, {:trust_policy, :known_identity, reason}}
    end
  end

  @doc """
  Check contact identity using known identities storage via `Ockam.Identity.TrustPolicy.KnownIdentities` module

  If the contact is not present in known identities - add it as a new contact
  If the contact with the same ID exists in known identities - check the idenity history
    If history is equal - contact is trusted
    If history is newer - update the known contact
    If history is older of in conflict - refuse to trust the contact
  """
  def cached_identity(
        _my_info,
        %{id: contact_id, identity: contact},
        known_identities_mod,
        extra_args \\ []
      ) do
    case known_identities_mod.get_identity(contact_id, extra_args) do
      {:ok, known_contact} ->
        case Identity.compare_identity_change_history(contact, known_contact) do
          {:ok, :equal} ->
            :ok

          {:ok, :newer} ->
            known_identities_mod.set_identity(contact_id, contact, extra_args)

          {:ok, :conflict} ->
            {:error,
             {:trust_policy, :cached_identity, {:identity_conflict, contact, known_contact}}}

          {:ok, :older} ->
            {:error,
             {:trust_policy, :cached_identity, {:identity_is_old, contact, known_contact}}}

          {:error, err} ->
            {:error, {:trust_policy, :cached_identity, {:api_error, err}}}
        end

      {:error, :not_found} ->
        known_identities_mod.set_identity(contact_id, contact, extra_args)

      {:error, reason} ->
        {:error, {:trust_policy, :cached_identity, reason}}
    end
  end

  defp apply_rule({module, function, extra_args}, my_info, contact_info) do
    args = [my_info, contact_info | extra_args]
    apply(module, function, args)
  end

  def run_fun(my_info, contact_info, function, extra_args)
      when is_function(function, length(extra_args) + 2) do
    args = [my_info, contact_info | extra_args]
    apply(function, args)
  end

  defp expand_config(config) do
    Enum.map(
      config,
      fn
        function when is_atom(function) ->
          {__MODULE__, function, []}

        function when is_function(function, 2) ->
          {__MODULE__, :run_fun, [function, []]}

        {function, args} when is_list(args) and is_function(function, length(args) + 2) ->
          {__MODULE__, :run_fun, [function, args]}

        {function, args} when is_atom(function) and is_list(args) ->
          {__MODULE__, function, args}

        {module, function, args} when is_atom(module) and is_atom(function) and is_list(args) ->
          {module, function, args}
      end
    )
  end
end

defmodule Ockam.Identity.TrustPolicy.KnownIdentities do
  @moduledoc """
  Behaviour to implement modules to manage trust policy known identities table
  """
  @callback get_identity(contact_id :: binary(), args :: list()) ::
              {:ok, contact :: binary()} | {:error, :not_found} | {:error, reason :: any()}
  @callback set_identity(contact_id :: binary(), contact :: binary(), args :: list()) ::
              :ok | {:error, reason :: any()}
end

defmodule Ockam.Identity.TrustPolicy.KnownIdentitiesEts do
  @moduledoc """
  Trust policy known identities table implemented with ETS table

  **WARNING: NOT FOR PRODUCTION USE**
  ETS table is not persistent and created on request by the current process.
  """
  @behaviour Ockam.Identity.TrustPolicy.KnownIdentities

  @table __MODULE__
  def get_identity(contact_id, _args) do
    ensure_table()

    case :ets.lookup(@table, contact_id) do
      [] -> {:error, :not_found}
      [{^contact_id, contact}] -> {:ok, contact}
    end
  end

  def set_identity(contact_id, contact, _args) do
    ensure_table()
    true = :ets.insert(@table, {contact_id, contact})
    {:ok, contact}
  end

  def ensure_table() do
    case :ets.info(@table) do
      :undefined -> :ets.new(@table, [:public, :named_table])
      _table -> :ok
    end
  end
end
