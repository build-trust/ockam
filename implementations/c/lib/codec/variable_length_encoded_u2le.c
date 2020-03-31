#include <stdint.h>

#define HEX_08 (uint8_t)0x08
#define HEX_FF (uint8_t)0xFF

/**
 * decode_variable_length_encoded_u2le
 * @param encoded [in] - buffer of encoded bytes
 * @param val [out] - decoded value
 * @return - encoded + number of bytes decoded
 */
uint8_t* decode_variable_length_encoded_u2le(uint8_t* encoded, uint16_t* val) {
  uint8_t ls_byte = *encoded++;
  uint8_t ms_byte = 0;

  if ((0 == encoded) || (0 == val)) {
    encoded = 0;
    goto exit_block;
  }

  if ((ls_byte & 0x80u) != 0) {
    ms_byte = *encoded++;
  }
  *val = (ms_byte << 0x07u) + (ls_byte & 0x7fu);

exit_block:
  return encoded;
}

/**
 * encode_variable_length_encoded_u2le
 * @param encoded [out] - buffer to receive encoded bytes
 * @param val [in] - value to encode, must be < 0x7fff
 * @return - encoded + bytes encoded
 */
uint8_t* encode_variable_length_encoded_u2le(uint8_t* encoded, uint16_t val) {
  uint8_t ls_byte = val & HEX_FF;
  uint8_t ms_byte = val >> HEX_08;

  if ((val & (uint16_t)0x8000) || (0 == encoded)) {
    encoded = 0;
    goto exit_block;
  }

  if (val < 0x80u) {
    *encoded++ = ls_byte;
  } else {
    ms_byte = (ms_byte << 0x01u) + ((ls_byte & 0x80u) ? 1 : 0);
    ls_byte |= 0x80u;
    *encoded++ = ls_byte;
    *encoded++ = ms_byte;
  }

exit_block:
  return encoded;
}