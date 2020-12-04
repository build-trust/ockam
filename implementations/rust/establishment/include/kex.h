//
// Created by Oleksandr Deundiak on 10.09.2020.
//

#ifndef RUST_KEY_EXCHANGE_H
#define RUST_KEY_EXCHANGE_H

#ifdef __cplusplus
extern "C" {
#endif

typedef uint64_t ockam_vault_t;
typedef uint64_t ockam_establishment_initiator_t;
typedef uint64_t ockam_establishment_responder_t;
typedef uint64_t ockam_establishment_t;
typedef uint64_t ockam_vault_secret_t;

uint32_t ockam_establishment_xx_initiator(ockam_establishment_initiator_t* establishment_initiator, ockam_vault_t vault);
uint32_t ockam_establishment_xx_responder(ockam_establishment_responder_t* establishment_responder, ockam_vault_t vault);

uint32_t ockam_establishment_xx_initiator_encode_message_1(ockam_establishment_initiator_t establishment_initiator,
                                                 const uint8_t* payload,
                                                 size_t payload_length,
                                                 uint8_t* m1,
                                                 size_t m1_size,
                                                 size_t* m1_length);

uint32_t ockam_establishment_xx_responder_encode_message_2(ockam_establishment_responder_t establishment_responder,
                                                 const uint8_t* payload,
                                                 size_t payload_length,
                                                 uint8_t* m2,
                                                 size_t m2_size,
                                                 size_t* m2_length);

uint32_t ockam_establishment_xx_initiator_encode_message_3(ockam_establishment_initiator_t establishment_initiator,
                                                 const uint8_t* payload,
                                                 size_t payload_length,
                                                 uint8_t* m3,
                                                 size_t m3_size,
                                                 size_t* m3_length);

uint32_t ockam_establishment_xx_responder_decode_message_1(ockam_establishment_responder_t establishment_responder,
                                                 const uint8_t* m1,
                                                 size_t m1_length);

uint32_t ockam_establishment_xx_initiator_decode_message_2(ockam_establishment_initiator_t establishment_initiator,
                                                 const uint8_t* m2,
                                                 size_t m2_length);

uint32_t ockam_establishment_xx_responder_decode_message_3(ockam_establishment_responder_t establishment_responder,
                                                 const uint8_t* m3,
                                                 size_t m3_length);

uint32_t ockam_establishment_xx_initiator_finalize(ockam_establishment_initiator_t establishment_initiator,
                                         ockam_establishment_t* establishment);

uint32_t ockam_establishment_xx_responder_finalize(ockam_establishment_responder_t establishment_responder,
                                         ockam_establishment_t* establishment);

#ifdef __cplusplus
} // extern "C"
#endif

#endif //RUST_KEY_EXCHANGE_H
