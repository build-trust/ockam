defmodule Ockam.Channel.Handshake do
  alias Ockam.Channel.CipherState
  alias Ockam.Channel.HashState
  alias Ockam.Channel.Protocol
  alias Ockam.Channel
  alias Ockam.Vault
  alias Ockam.Vault.KeyPair

  @type role :: :initiator | :responder
  @type token :: :s | :e | :ee | :ss | :es | :se
  @type keypair :: Ockam.Vault.KeyPair.t()
  @type dh :: :x25519

  defstruct ss: nil,
            s: nil,
            e: nil,
            rs: nil,
            re: nil,
            role: :initiator,
            dh: :x25519,
            msgs: []

  def init(protocol_name, role, prologue, keys) when is_binary(protocol_name) do
    with {:ok, protocol} <- Protocol.from_name(protocol_name) do
      init(protocol, role, prologue, keys)
    end
  end

  def init(protocol, role, prologue, {s, e, rs, re}) do
    ss =
      protocol
      |> HashState.init()
      |> HashState.mix_hash(prologue)

    base_handshake = %__MODULE__{
      ss: ss,
      s: s,
      e: e,
      rs: rs,
      re: re,
      role: role,
      dh: Protocol.dh(protocol),
      msgs: Protocol.msgs(role, protocol)
    }

    handshake =
      role
      |> Protocol.pre_msgs(protocol)
      |> Enum.reduce(base_handshake, fn
        {:out, [:s]}, hs -> mix_hash(hs, KeyPair.public_key(s))
        {:out, [:e]}, hs -> mix_hash(hs, KeyPair.public_key(e))
        {:in, [:s]}, hs -> mix_hash(hs, KeyPair.public_key(rs))
        {:in, [:e]}, hs -> mix_hash(hs, KeyPair.public_key(re))
      end)

    {:ok, handshake}
  end

  def finalize(%__MODULE__{msgs: [], ss: ss, role: role} = hs) do
    {c1, c2} = HashState.split(ss)
    chan = %Channel{hash: HashState.h(ss), state: hs}

    case role do
      :initiator -> {:ok, %Channel{chan | tx: c2, rx: c1}}
      :responder -> {:ok, %Channel{chan | tx: c1, rx: c2}}
    end
  end

  def finalize(%__MODULE__{}), do: {:error, {__MODULE__, :invalid_finalize_state}}

  def next_message(%__MODULE__{msgs: [{dir, _} | _]}), do: dir
  def next_message(%__MODULE__{msgs: []}), do: :done

  def write_message(%__MODULE__{msgs: [{:out, msg} | msgs]} = hs, payload) do
    {hs, msgbuf1} = write_message(%__MODULE__{hs | msgs: msgs}, msg, "")
    {:ok, hs, msgbuf2} = encrypt_and_hash(hs, payload)
    {:ok, hs, <<msgbuf1::binary, msgbuf2::binary>>}
  end

  defp write_message(%__MODULE__{} = hs, [], msgbuf), do: {hs, msgbuf}

  defp write_message(%__MODULE__{} = hs, [tok | toks], msgbuf) do
    {hs, msgbuf1} = write_token(hs, tok)
    write_message(hs, toks, <<msgbuf::binary, msgbuf1::binary>>)
  end

  def read_message(%__MODULE__{msgs: [{:in, msg} | msgs]} = hs, message) do
    with {:ok, hs, rest} <- read_message(%__MODULE__{hs | msgs: msgs}, msg, message) do
      decrypt_and_hash(hs, rest)
    end
  end

  defp read_message(%__MODULE__{} = hs, [], data), do: {:ok, hs, data}

  defp read_message(%__MODULE__{} = hs, [tok | toks], data) do
    with {:ok, hs, data} <- read_token(hs, tok, data) do
      read_message(hs, toks, data)
    end
  end

  def remote_keys(%__MODULE__{rs: rs}), do: rs

  defp write_token(%__MODULE__{e: nil} = hs, :e) do
    e = new_key_pair(hs)
    pub_e = KeyPair.public_key(e)
    {mix_hash(%__MODULE__{hs | e: e}, pub_e), pub_e}
  end

  # Should only apply during test
  defp write_token(%__MODULE__{e: e} = hs, :e) do
    pub_e = KeyPair.public_key(e)
    {mix_hash(hs, pub_e), pub_e}
  end

  defp write_token(%__MODULE__{s: s} = hs, :s) do
    {:ok, hs, msg} = encrypt_and_hash(hs, KeyPair.public_key(s))
    {hs, msg}
  end

  defp write_token(%__MODULE__{} = hs, token) do
    {k1, k2} = dh_token(hs, token)
    {mix_key(hs, dh(hs, k1, k2)), <<>>}
  end

  defp read_token(%__MODULE__{dh: dh} = hs, :e, data) do
    dh_len = Vault.dh_length(dh)

    case data do
      <<re_pub::size(dh_len)-binary, data1::binary>> ->
        re = KeyPair.new(dh, public: re_pub)
        {:ok, mix_hash(%__MODULE__{hs | re: re}, re_pub), data1}

      _other ->
        {:error, {__MODULE__, {:bad_data, {:expected_token, :e, dh_len}}}}
    end
  end

  defp read_token(%__MODULE__{dh: dh} = hs, :s, data) do
    dh_len =
      if has_key(hs) do
        Vault.dh_length(dh) + 16
      else
        Vault.dh_length(dh)
      end

    case data do
      <<temp::size(dh_len)-binary, data1::binary>> ->
        with {:ok, hs, rs_pub} <- decrypt_and_hash(hs, temp) do
          rs = KeyPair.new(dh, public: rs_pub)
          {:ok, %__MODULE__{hs | rs: rs}, data1}
        end

      _ ->
        {:error, {__MODULE__, {:expected_token, :s, dh_len}}}
    end
  end

  defp read_token(%__MODULE__{} = hs, token, data) do
    {k1, k2} = dh_token(hs, token)
    {:ok, mix_key(hs, dh(hs, k1, k2)), data}
  end

  defp dh_token(%__MODULE__{e: e, re: re}, :ee), do: {e, re}
  defp dh_token(%__MODULE__{e: e, rs: rs, role: :initiator}, :es), do: {e, rs}
  defp dh_token(%__MODULE__{s: s, re: re, role: :responder}, :es), do: {s, re}
  defp dh_token(%__MODULE__{s: s, re: re, role: :initiator}, :se), do: {s, re}
  defp dh_token(%__MODULE__{e: e, rs: rs, role: :responder}, :se), do: {e, rs}
  defp dh_token(%__MODULE__{s: s, rs: rs}, :ss), do: {s, rs}

  defp new_key_pair(%__MODULE__{dh: dh}), do: KeyPair.new(dh)

  defp dh(%__MODULE__{dh: dh}, key1, key2), do: Vault.dh(dh, key1, key2)

  defp has_key(%__MODULE__{ss: ss}) do
    ss
    |> HashState.cipher_state()
    |> CipherState.has_key()
  end

  defp mix_key(%__MODULE__{ss: ss} = hs, data) do
    %__MODULE__{hs | ss: HashState.mix_key(ss, data)}
  end

  defp mix_hash(%__MODULE__{ss: ss} = hs, data) do
    %__MODULE__{hs | ss: HashState.mix_hash(ss, data)}
  end

  defp encrypt_and_hash(%__MODULE__{ss: ss} = hs, plaintext) do
    {:ok, ss, ciphertext} = HashState.encrypt_and_hash(ss, plaintext)
    {:ok, %__MODULE__{hs | ss: ss}, ciphertext}
  end

  defp decrypt_and_hash(%__MODULE__{ss: ss} = hs, ciphertext) do
    with {:ok, ss, plaintext} <- HashState.decrypt_and_hash(ss, ciphertext) do
      {:ok, %__MODULE__{hs | ss: ss}, plaintext}
    else
      other ->
        {:error, {:decrypt_and_hash, other}}
    end
  end
end
