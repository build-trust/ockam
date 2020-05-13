/**
 * @file  impl.h
 * @brief The interface for a memory implementation
 */

#ifndef OCKAM_MEMORY_IMPL_H_
#define OCKAM_MEMORY_IMPL_H_

/**
 * @struct  ockam_memory_dispatch_table
 * @brief   The Ockam Memory implementation functions
 */
typedef struct {
  /**
   * @brief   Deinitialize the specified ockam memory object.
   * @param   memory[in]  The ockam memory object to deinitialize.
   * @return  OCKAM_ERROR_NONE on success.
   * @return  MEMORY_ERROR_INVALID_PARAM if invalid memory received.
   */
  ockam_error_t (*deinit)(ockam_memory_t* memory);

  /**
   * @brief   Allocate memory from the specified memory module
   * @param   memory[in]      The ockam memory object to use.
   * @param   buffer[out]     Pointer to a buffer to allocate.
   * @param   buffer_size[in] Buffer size (in bytes) to allocate.
   * @return  OCKAM_ERROR_NONE on success.
   * @return  MEMORY_ERROR_INVALID_PARAM if invalid memory or buffer received.
   * @return  MEMORY_ERROR_INVALID_SIZE if buffer_size <=0.
   */
  ockam_error_t (*alloc_zeroed)(ockam_memory_t* memory, void** buffer, size_t buffer_size);

  /**
   * @brief   Free the specified buffer.
   * @param   memory[in]      The ockam memory object to use.
   * @param   buffer[in]      Buffer to free. Must have been allocated from alloc_zeroed().
   * @param   buffer_size[in] Size of the buffer that was allocated. Must match what was specified in alloc_zeroed().
   * @return  OCKAM_ERROR_NONE on success.
   * @return  MEMORY_ERROR_INVALID_PARAM if invalid memory or buffer received.
   * @return  MEMORY_ERROR_INVALID_SIZE if buffer_size <=0.
   */
  ockam_error_t (*free)(ockam_memory_t* memory, void* buffer, size_t buffer_size);

  /**
   * @brief   Set a set_size number of bytes to the buffer with value
   * @param   memory[in]    The ockam memory object to use.
   * @param   buffer[out]   The buffer to fill with the specified value.
   * @param   value[in]     The value to set the the buffer with.
   * @param   set_size[in]  The number of bytes to set buffer with value.
   * @return  OCKAM_ERROR_NONE on success.
   * @return  MEMORY_ERROR_INVALID_PARAM if invalid memory or buffer received.
   * @return  MEMORY_ERROR_INVALID_SIZE if set_size <=0.
   */
  ockam_error_t (*set)(ockam_memory_t* memory, void* buffer, uint8_t value, size_t set_size);

  /**
   * @brief   Copy data from the source buffer to the destination buffer
   * @param   memory[in]        The ockam memory object to use.
   * @param   destination[out]  Buffer to place the copied data into. Can not overlap with source.
   * @param   source[in]        Buffer to copy data from.
   * @param   copy_size[in]     Size of data to copy.
   * @return  OCKAM_ERROR_NONE on success.
   * @return  MEMORY_ERROR_INVALID_PARAM if invalid memory, destination or source received.
   * @return  MEMORY_ERROR_INVALID_SIZE if copy_size <=0.
   */
  ockam_error_t (*copy)(ockam_memory_t* memory, void* destination, const void* source, size_t copy_size);

  /**
   * @brief   Move a move_size bytes from source to destination.
   * @param   memory[in]        The ockam memory object to use.
   * @param   destination[out]  Buffer to place the moved data into. May overlap with source.
   * @param   source[in]        Buffer to move data from.
   * @param   move_size[in]     Size of data to move.
   * @return  OCKAM_ERROR_NONE on success.
   * @return  MEMORY_ERROR_INVALID_PARAM if invalid memory, destination or source received.
   * @return  MEMORY_ERROR_INVALID_SIZE if move_size <=0.
   */
  ockam_error_t (*move)(ockam_memory_t* memory, void* destination, void* source, size_t move_size);
} ockam_memory_dispatch_table_t;

/**
 * @struct  ockam_memory_t
 * @brief   The ockam memory object struct.
 */
struct ockam_memory_t {
  ockam_memory_dispatch_table_t* dispatch;
  void*                          context;
};

#endif
