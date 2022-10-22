/**
 * @file  impl.h
 * @brief The interface for a random implementation
 */

#ifndef OCKAM_RANDOM_IMPL_H_
#define OCKAM_RANDOM_IMPL_H_

/**
 * @struct  ockam_random_dispatch_table
 * @brief   The Ockam Random implementation functions
 */
typedef struct {
  /**
   * @brief   Deinitialize the specified ockam random object.
   * @param   random[in]  The ockam random object to deinitialize.
   * @return  OCKAM_ERROR_NONE on success.
   * @return  OCKAM_RANDOM_ERROR_INVALID_PARAM if invalid random received.
   */
  ockam_error_t (*deinit)(ockam_random_t* random);

  /**
   * @brief   Retrieve a specified number of random bytes from the random implementation.
   * @param   random[in]      The ockam random object to use.
   * @param   buffer[out]     Buffer to fill with random bytes.
   * @param   buffer_size[in] Buffer size (in bytes).
   * @return  OCKAM_ERROR_NONE on success.
   * @return  OCKAM_RANDOM_ERROR_INVALID_PARAM if invalid random or buffer received.
   * @return  OCKAM_RANDOM_ERROR_INVALID_SIZE if buffer_size <=0.
   * @return  OCKAM_RANDOM_ERROR_GET_BYTES_FAIL if unable to retrieve random bytes.
   */
  ockam_error_t (*get_bytes)(ockam_random_t* random, uint8_t* buffer, size_t buffer_size);
} ockam_random_dispatch_table_t;

/**
 * @struct  ockam_random_t
 * @brief   The ockam random object struct.
 */
struct ockam_random_t {
  ockam_random_dispatch_table_t* dispatch;
  void*                          context;
};

#endif
