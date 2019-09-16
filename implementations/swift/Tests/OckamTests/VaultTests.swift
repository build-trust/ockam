import XCTest
@testable import Ockam

final class VaultTests: XCTestCase {

  func testGenerate() {
    XCTAssertEqual(Vault.generateEphemeralKeyPair(), "a generated key")
  }

}
