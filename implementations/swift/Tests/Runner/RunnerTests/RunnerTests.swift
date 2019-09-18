import XCTest
import CryptoKit

@testable import Runner
@testable import Ockam

extension Data {
  /// A hexadecimal string representation of the bytes.
  func hexEncodedString() -> String {
    let hexDigits = Array("0123456789abcdef".utf16)
    var hexChars = [UTF16.CodeUnit]()
    hexChars.reserveCapacity(count * 2)

    for byte in self {
      let (index1, index2) = Int(byte).quotientAndRemainder(dividingBy: 16)
      hexChars.append(hexDigits[index1])
      hexChars.append(hexDigits[index2])
    }

    return String(utf16CodeUnits: hexChars, count: hexChars.count)
  }
}

extension String {
  /// A data representation of the hexadecimal bytes in this string.
  func hexDecodedData() -> Data {
    // Get the UTF8 characters of this string
    let chars = Array(utf8)

    // Keep the bytes in an UInt8 array and later convert it to Data
    var bytes = [UInt8]()
    bytes.reserveCapacity(count / 2)

    // It is a lot faster to use a lookup map instead of strtoul
    let map: [UInt8] = [
      0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, // 01234567
      0x08, 0x09, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // 89:;<=>?
      0x00, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x00, // @ABCDEFG
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00  // HIJKLMNO
    ]

    // Grab two characters at a time, map them and turn it into a byte
    for i in stride(from: 0, to: count, by: 2) {
      let index1 = Int(chars[i] & 0x1F ^ 0x10)
      let index2 = Int(chars[i + 1] & 0x1F ^ 0x10)
      bytes.append(map[index1] << 4 | map[index2])
    }

    return Data(bytes)
  }
}

extension ContiguousBytes {
    /// A Data instance created safely from the contiguous bytes without making any copies.
    var dataRepresentation: Data {
        return self.withUnsafeBytes { bytes in
            let cfdata = CFDataCreateWithBytesNoCopy(nil, bytes.baseAddress?.assumingMemoryBound(to: UInt8.self), bytes.count, kCFAllocatorNull)
            return ((cfdata as NSData?) as Data?) ?? Data()
        }
    }
}

extension SharedSecret {
  init<D>(rawRepresentation data: D) throws where D: ContiguousBytes {
    try self.init(rawRepresentation: data)
  }
}

class RunnerTests: XCTestCase {

    func testEncrypt() {

      // http://luca-giuzzi.unibs.it/corsi/Support/papers-cryptography/gcm-spec.pdf
      // TestCase 4 from GCM Spec
      let K = "feffe9928665731c6d6a8f9467308308".hexDecodedData()
      let P = """
      d9313225f88406e5a55909c5aff5269a\
      86a7a9531534f7da2e4c303d8a318a72\
      1c3c0c95956809532fcf0e2449a6b525\
      b16aedf5aa0de657ba637b39
      """.hexDecodedData()
      let A = "feedfacedeadbeeffeedfacedeadbeefabaddad2".hexDecodedData()
      let IV = "cafebabefacedbaddecaf888".hexDecodedData()
      let C = """
      42831ec2217774244b7221b784d0d49c\
      e3aa212f2c02a4e035c17e2329aca12e\
      21d514b25466931c7d8f6a5aac84aa05\
      1ba30b396a0aac973d58e091
      """.hexDecodedData()
      let T = "5bc94fbc3221a5db94fae95ae7121a47".hexDecodedData()

      let encrypted = try! Vault.encrypt(P, using: K, nonce: IV, authenticating: A)

      XCTAssertEqual(encrypted, (IV + C + T))
    }

    func testDecrypt() {

      // http://luca-giuzzi.unibs.it/corsi/Support/papers-cryptography/gcm-spec.pdf
      // TestCase 4 from GCM Spec
      let K = "feffe9928665731c6d6a8f9467308308".hexDecodedData()
      let P = """
      d9313225f88406e5a55909c5aff5269a\
      86a7a9531534f7da2e4c303d8a318a72\
      1c3c0c95956809532fcf0e2449a6b525\
      b16aedf5aa0de657ba637b39
      """.hexDecodedData()
      let A = "feedfacedeadbeeffeedfacedeadbeefabaddad2".hexDecodedData()
      let IV = "cafebabefacedbaddecaf888".hexDecodedData()
      let C = """
      42831ec2217774244b7221b784d0d49c\
      e3aa212f2c02a4e035c17e2329aca12e\
      21d514b25466931c7d8f6a5aac84aa05\
      1ba30b396a0aac973d58e091
      """.hexDecodedData()
      let T = "5bc94fbc3221a5db94fae95ae7121a47".hexDecodedData()

      let decrypted = try! Vault.decrypt((IV + C + T), using: K, authenticating: A)

      XCTAssertEqual(decrypted, P)
    }

    func testGenerateSharedSecret() {
      let alicePrivateKey = P256.KeyAgreement.PrivateKey()
      let bobPrivateKey = P256.KeyAgreement.PrivateKey()

      let aliceSharedSecret = try! alicePrivateKey.sharedSecretFromKeyAgreement(with: bobPrivateKey.publicKey)
      let bobSharedSecret = try! bobPrivateKey.sharedSecretFromKeyAgreement(with: alicePrivateKey.publicKey)

      XCTAssertEqual(aliceSharedSecret, bobSharedSecret)
    }

    func testGenerateSharedSecret1() {

      let alicePrivateKey_x963Representation = """
      042e854735a89035e0c53b02e5ba3cad772131c\
      08bfec9fa12b90f1e106a1dc7d5535e64720577\
      512e0f495209c7b39149db5bfa402f2dbc5683f\
      3db8685dc75e8e829272ce8d967c0dde7edeb4a\
      c2b8e24b804926df5a2fd063f5e620de339f64
      """.hexDecodedData()

      let bobPrivateKey_x963Representation = """
      04c3c8900003b2529c00a9ba9b6843a86576a88\
      614cff067f7bd83763e7473c8507ac88ed0198e\
      839998483e5e067b9a208b103a2d0a2e56d9827\
      cfe8bed90faa5d35e7399864a9e52606b279bf8\
      eb275b64fe6828fb28103bafdbb5bb840b5cb5
      """.hexDecodedData()

      let expectedSharedSecret = "27d76bbb2dacb3006bc64c637130c5651a1c87749404e4fb0715a983a0ced199".hexDecodedData()

      let alicePrivateKey = try! P256.KeyAgreement.PrivateKey(x963Representation: alicePrivateKey_x963Representation)
      let bobPrivateKey = try! P256.KeyAgreement.PrivateKey(x963Representation: bobPrivateKey_x963Representation)

      let aliceSharedSecret = try! alicePrivateKey.sharedSecretFromKeyAgreement(with: bobPrivateKey.publicKey)
      print("aliceSharedSecret: ", aliceSharedSecret.dataRepresentation.hexEncodedString())

      let bobSharedSecret = try! bobPrivateKey.sharedSecretFromKeyAgreement(with: alicePrivateKey.publicKey)
      print("bobSharedSecret: ", bobSharedSecret.dataRepresentation.hexEncodedString())

      XCTAssertEqual(aliceSharedSecret, bobSharedSecret)
      XCTAssertEqual(expectedSharedSecret, aliceSharedSecret.dataRepresentation)
      XCTAssertEqual(expectedSharedSecret, bobSharedSecret.dataRepresentation)
    }

    func testDerivedKeys() {

      let alicePrivateKey_x963Representation = """
      042e854735a89035e0c53b02e5ba3cad772131c\
      08bfec9fa12b90f1e106a1dc7d5535e64720577\
      512e0f495209c7b39149db5bfa402f2dbc5683f\
      3db8685dc75e8e829272ce8d967c0dde7edeb4a\
      c2b8e24b804926df5a2fd063f5e620de339f64
      """.hexDecodedData()

      let bobPrivateKey_x963Representation = """
      04c3c8900003b2529c00a9ba9b6843a86576a88\
      614cff067f7bd83763e7473c8507ac88ed0198e\
      839998483e5e067b9a208b103a2d0a2e56d9827\
      cfe8bed90faa5d35e7399864a9e52606b279bf8\
      eb275b64fe6828fb28103bafdbb5bb840b5cb5
      """.hexDecodedData()

      let protocolSalt = "fb49c1746873c7f97b8f245bdf77dbd8".hexDecodedData()
      let expectedDerivedKey = "2cfe0ecb2efbad221e1c1b336ca0039ad91f601037fcd964d31e22dfbac0ee68".hexDecodedData()

      let alicePrivateKey = try! P256.KeyAgreement.PrivateKey(x963Representation: alicePrivateKey_x963Representation)
      let bobPrivateKey = try! P256.KeyAgreement.PrivateKey(x963Representation: bobPrivateKey_x963Representation)

      let aliceSharedSecret = try! alicePrivateKey.sharedSecretFromKeyAgreement(with: bobPrivateKey.publicKey)
      print("aliceSharedSecret: ", aliceSharedSecret.dataRepresentation.hexEncodedString())

      let aliceDerivedKey = aliceSharedSecret.hkdfDerivedSymmetricKey(
        using: SHA256.self,
        salt: protocolSalt,
        sharedInfo: bobPrivateKey.publicKey.rawRepresentation + alicePrivateKey.publicKey.rawRepresentation,
        outputByteCount: 16
      )
      print("aliceDerivedKey: ", aliceDerivedKey.dataRepresentation.hexEncodedString())

      let bobSharedSecret = try! bobPrivateKey.sharedSecretFromKeyAgreement(with: alicePrivateKey.publicKey)
      print("bobSharedSecret: ", bobSharedSecret.dataRepresentation.hexEncodedString())

      let bobDerivedKey = bobSharedSecret.hkdfDerivedSymmetricKey(
        using: SHA256.self,
        salt: protocolSalt,
        sharedInfo: bobPrivateKey.publicKey.rawRepresentation + alicePrivateKey.publicKey.rawRepresentation,
        outputByteCount: 16
      )
      print("bobDerivedKey: ", bobDerivedKey.dataRepresentation.hexEncodedString())

      XCTAssertEqual(aliceSharedSecret, bobSharedSecret)
      XCTAssertEqual(aliceDerivedKey, bobDerivedKey)
      XCTAssertEqual(expectedDerivedKey, aliceDerivedKey.dataRepresentation)
      XCTAssertEqual(expectedDerivedKey, bobDerivedKey.dataRepresentation)
    }

}
