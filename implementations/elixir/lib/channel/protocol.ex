defmodule Ockam.Channel.Protocol do
  @moduledoc "Represents a Noise protocol configuration"

  @type noise_pattern ::
          :nn | :kn | :nk | :kk | :nx | :kx | :xn | :in | :xk | :ik | :xx | :ix | :psk
  @type noise_msg :: {:in | :out, [Ockam.Channel.Handshake.token()]}

  defstruct pattern: :xx, dh: :x25519, cipher: :aes_256_gcm, hash: :sha256

  @type t :: %__MODULE__{
          pattern: noise_pattern(),
          dh: Ockam.Channel.Handshake.curve(),
          cipher: Ockam.Channel.CipherState.cipher(),
          hash: Ockam.Channel.HashState.hash()
        }

  def cipher(%__MODULE__{cipher: cipher}), do: cipher
  def dh(%__MODULE__{dh: dh}), do: dh
  def hash(%__MODULE__{hash: hash}), do: hash
  def pattern(%__MODULE__{pattern: pattern}), do: pattern

  def name(%__MODULE__{pattern: pattern, dh: dh, cipher: cipher, hash: hash}) do
    to_name(pattern, dh, cipher, hash)
  end

  def from_name(<<"Noise_", rest::binary>>) do
    do_from_name(rest)
  end

  def from_name(<<"NoisePSK_", rest::binary>>) do
    do_from_name(rest)
  end

  defp do_from_name(rest) do
    case String.split(rest, "_", parts: 4) do
      [pattern_s, dh_s, cipher_s, hash_s] ->
        with {:ok, pattern} <- parse_pattern(pattern_s),
             {:ok, dh} <- parse_dh(dh_s),
             {:ok, cipher} <- parse_cipher(cipher_s),
             {:ok, hash} <- parse_hash(hash_s) do
          if supported(pattern, dh, cipher, hash) do
            {:ok,
             %__MODULE__{
               pattern: pattern,
               dh: dh,
               cipher: cipher,
               hash: hash
             }}
          else
            {:error, {__MODULE__, :unsupported_pattern}}
          end
        end

      _ ->
        {:error, {__MODULE__, :unrecognized_name}}
    end
  end

  def msgs(role, %__MODULE__{pattern: pattern}) do
    {_pre, msgs} = protocol(pattern)
    role_adapt(role, msgs)
  end

  def pre_msgs(role, %__MODULE__{pattern: pattern}) do
    {pre, _msgs} = protocol(pattern)
    role_adapt(role, pre)
  end

  ## Private

  defp to_name(pattern, dh, cipher, hash) do
    <<"Noise_", pattern_name(pattern)::binary, "_", dh_name(dh)::binary, "_",
      cipher_name(cipher)::binary, "_", hash_name(hash)::binary>>
  end

  defp pattern_name(pattern) when is_atom(pattern) do
    [simple | rest] =
      pattern
      |> Atom.to_string()
      |> String.split("_", parts: 2)

    case rest do
      [] -> String.upcase(simple)
      [bin] -> <<String.upcase(simple)::binary, "+", bin::binary>>
    end
  end

  defp parse_pattern(pattern) when is_binary(pattern) do
    [init | mod2] = String.split(pattern, "+", parts: 2)
    [simple | mod1] = String.split(init, ~r/[^A-Z]/, parts: 2)
    simple = String.downcase(simple)

    case {mod1, mod2} do
      {[], _} -> {:ok, String.to_existing_atom(simple)}
      {[mod1s], [mod2s]} -> {:ok, String.to_existing_atom(simple <> "_" <> mod1s <> "_" <> mod2s)}
      {[mod1s], []} -> {:ok, String.to_existing_atom(simple <> "_" <> mod1s)}
    end
  end

  defp dh_name(:x25519), do: "25519"
  defp dh_name(:x448), do: "448"

  defp parse_dh("25519"), do: {:ok, :x25519}
  defp parse_dh("448"), do: {:ok, :x448}

  defp cipher_name(:aes_256_gcm), do: "AESGCM"
  defp cipher_name(:chachapoly), do: "ChaChaPoly"

  defp parse_cipher("AESGCM"), do: {:ok, :aes_256_gcm}
  defp parse_cipher("ChaChaPoly"), do: {:ok, :chachapoly}

  defp hash_name(:sha256), do: "SHA256"
  defp hash_name(:sha512), do: "SHA512"
  defp hash_name(:blake2s), do: "BLAKE2s"
  defp hash_name(:blake2b), do: "BLAKE2b"

  defp parse_hash(hash) when is_binary(hash) do
    atom =
      hash
      |> String.downcase()
      |> String.to_existing_atom()

    {:ok, atom}
  end

  defp role_adapt(:initiator, msgs), do: msgs

  defp role_adapt(:responder, msgs) do
    Enum.map(msgs, fn
      {:in, msg} -> {:out, msg}
      {:out, msg} -> {:in, msg}
    end)
  end

  defp protocol(:nn) do
    {[], [{:out, [:e]}, {:in, [:e, :ee]}]}
  end

  defp protocol(:kn) do
    {[{:out, [:s]}], [{:out, [:e]}, {:in, [:e, :ee, :se]}]}
  end

  defp protocol(:nk) do
    {[{:in, [:s]}], [{:out, [:e, :es]}, {:in, [:e, :ee]}]}
  end

  defp protocol(:kk) do
    {[{:out, [:s]}, {:in, [:s]}], [{:out, [:e, :es, :ss]}, {:in, [:e, :ee, :se]}]}
  end

  defp protocol(:nx) do
    {[], [{:out, [:e]}, {:in, [:e, :ee, :s, :es]}]}
  end

  defp protocol(:kx) do
    {[{:out, [:s]}], [{:out, [:e]}, {:in, [:e, :ee, :se, :s, :es]}]}
  end

  defp protocol(:xn) do
    {[], [{:out, [:e]}, {:in, [:e, :ee]}, {:out, [:s, :se]}]}
  end

  defp protocol(:in) do
    {[], [{:out, [:e, :s]}, {:in, [:e, :ee, :se]}]}
  end

  defp protocol(:xk) do
    {[{:in, [:s]}], [{:out, [:e, :es]}, {:in, [:e, :ee]}, {:out, [:s, :se]}]}
  end

  defp protocol(:ik) do
    {[{:in, [:s]}], [{:out, [:e, :es, :s, :ss]}, {:in, [:e, :ee, :se]}]}
  end

  defp protocol(:xx) do
    {[], [{:out, [:e]}, {:in, [:e, :ee, :s, :es]}, {:out, [:s, :se]}]}
  end

  defp protocol(:ix) do
    {[], [{:out, [:e, :s]}, {:in, [:e, :ee, :se, :s, :es]}]}
  end

  defp supported(pattern, dh, cipher, hash) do
    with true <- pattern in [:xx],
         true <- dh in [:x25519],
         true <- cipher in [:aes_256_gcm],
         true <- hash in [:sha256] do
      true
    end
  end
end
