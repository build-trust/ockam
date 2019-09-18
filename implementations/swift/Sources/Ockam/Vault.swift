import Foundation
import CryptoKit

public enum Vault {

  // AES-GCM Authenticated Encryption
  public static func encrypt(
    _ message: Data,
    using symmetricKey: Data,
    nonce: Data,
    authenticating authenticatedData: Data
  ) throws -> Data {

    let sealedBox = try AES.GCM.seal(
      message,
      using: SymmetricKey(data: symmetricKey),
      nonce: AES.GCM.Nonce(data: nonce),
      authenticating: authenticatedData
    )

    return sealedBox.combined ?? Data()
  }

  public static func decrypt(
    _ message: Data,
    using symmetricKey: Data,
    authenticating authenticatedData: Data
  ) throws -> Data {

    let sealedBox = try AES.GCM.SealedBox(combined: message) // message = (nonce + ciphertext + tag)
    let decrypted = try AES.GCM.open(
      sealedBox,
      using: SymmetricKey(data: symmetricKey),
      authenticating: authenticatedData
    )

    return decrypted
  }

}
