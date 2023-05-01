defmodule Ockam.SecureChannel.EncryptedTransportProtocol.AeadAesGcm do
  @moduledoc false

  alias Ockam.Vault
  alias __MODULE__
  @max_nonce trunc(:math.pow(2, 64)) - 1

  defmodule Encryptor do
    @moduledoc false
    alias __MODULE__
    defstruct [:vault, :k, :nonce, :rekey_each]

    def new(vault, k, nonce), do: new(vault, k, nonce, 32)

    def new(vault, k, nonce, rekey_each) do
      %Encryptor{vault: vault, k: k, nonce: nonce, rekey_each: rekey_each}
    end

    def encrypt(
          ad,
          plaintext,
          %Encryptor{vault: vault, k: k, nonce: nonce, rekey_each: rekey_each} = state
        ) do
      with {:ok, ciphertext} <- Vault.aead_aes_gcm_encrypt(vault, k, nonce, ad, plaintext),
           {:ok, next_nonce} <- AeadAesGcm.increment_nonce(nonce),
           {:ok, next_k, _} <- rotate_if_needed(vault, next_nonce, k, rekey_each) do
        {:ok, <<nonce::unsigned-big-integer-size(64), ciphertext::binary>>,
         %Encryptor{state | nonce: next_nonce, k: next_k}}
      end
    end

    defp rotate_if_needed(vault, next_nonce, k, rekey_each) do
      if rem(next_nonce, rekey_each) == 0 do
        with {:ok, new_k} <- AeadAesGcm.rekey(vault, k) do
          :ok = Vault.secret_destroy(vault, k)
          {:ok, new_k, true}
        end
      else
        {:ok, k, false}
      end
    end
  end

  defmodule Decryptor do
    @moduledoc false
    alias __MODULE__
    defstruct [:vault, :k, :expected_nonce, :rekey_each, :prev_k, :seen, :prev_seen]

    def new(vault, k, nonce), do: new(vault, k, nonce, 32)

    def new(vault, k, nonce, rekey_each) do
      %Decryptor{
        vault: vault,
        k: k,
        expected_nonce: nonce,
        rekey_each: rekey_each,
        prev_k: nil,
        seen: MapSet.new(),
        prev_seen: MapSet.new()
      }
    end

    ## window_offset=0  this k window
    ## window_offset=-1 previous k window
    ## window_offset=1  next k window
    ## We don't allow out-of-order messages more than 1 rekey window away from the expected nonce
    defp decrypt_from(
           0,
           nonce,
           ad,
           ciphertext,
           %Decryptor{vault: vault, seen: seen, k: k, expected_nonce: expected_nonce} = state
         ) do
      if MapSet.member?(seen, nonce) do
        {:error, :repeated_nonce}
      else
        {:ok, next_nonce} = AeadAesGcm.increment_nonce(nonce)

        case Vault.aead_aes_gcm_decrypt(vault, k, nonce, ad, ciphertext) do
          {:ok, plaintext} ->
            {:ok, plaintext,
             %Decryptor{
               state
               | seen: MapSet.put(seen, nonce),
                 expected_nonce: max(expected_nonce, next_nonce)
             }}

          {:error, reason} ->
            {:error, reason}
        end
      end
    end

    defp decrypt_from(
           -1,
           nonce,
           ad,
           ciphertext,
           %Decryptor{vault: vault, prev_seen: prev_seen, prev_k: prev_k} = state
         ) do
      if MapSet.member?(prev_seen, nonce) do
        {:error, :repeated_nonce}
      else
        case Vault.aead_aes_gcm_decrypt(vault, prev_k, nonce, ad, ciphertext) do
          {:ok, plaintext} ->
            {:ok, plaintext, %Decryptor{state | prev_seen: MapSet.put(prev_seen, nonce)}}

          {:error, reason} ->
            {:error, reason}
        end
      end
    end

    defp decrypt_from(
           1,
           nonce,
           ad,
           ciphertext,
           %Decryptor{vault: vault, seen: seen, prev_k: prev_k, k: k} = state
         ) do
      {:ok, next_nonce} = AeadAesGcm.increment_nonce(nonce)
      {:ok, new_k} = AeadAesGcm.rekey(vault, k)

      case Vault.aead_aes_gcm_decrypt(vault, new_k, nonce, ad, ciphertext) do
        {:ok, plaintext} ->
          if prev_k != nil do
            :ok = Vault.secret_destroy(vault, prev_k)
          end

          {:ok, plaintext,
           %Decryptor{
             state
             | prev_k: k,
               k: new_k,
               prev_seen: seen,
               seen: MapSet.new([nonce]),
               expected_nonce: next_nonce
           }}

        {:error, reason} ->
          {:error, reason}
      end
    end

    defp decrypt_from(_n, _nonce, _ad, _ciphertext, _state) do
      {:error, :out_of_window}
    end

    def decrypt(
          ad,
          <<nonce::unsigned-big-integer-size(64), ciphertext::binary>>,
          state
        ) do
      # We can do this since nonce could never be below 0 (unsigned integer)
      window_offset =
        div(nonce, state.rekey_each) -
          div(max(0, state.expected_nonce - 1), state.rekey_each)

      decrypt_from(window_offset, nonce, ad, ciphertext, state)
    end
  end

  def increment_nonce(n) do
    case n + 1 do
      @max_nonce -> {:error, nil}
      valid_nonce -> {:ok, valid_nonce}
    end
  end

  def rekey(vault, k) do
    {:ok, <<new_k::binary-size(32), _::binary>>} =
      Vault.aead_aes_gcm_encrypt(vault, k, @max_nonce, <<>>, <<0::32*8>>)

    Vault.secret_import(vault, [type: :aes], new_k)
  end
end
