/**
 ********************************************************************************************************
 * @file    memory.h
 * @brief   Generic memory functions for the Ockam Library
 ********************************************************************************************************
 */

#ifndef OCKAM_MEMORY_H_
#define OCKAM_MEMORY_H_

/*
 ********************************************************************************************************
 * @defgroup    OCKAM_MEMORY OCKAM_MEMORY_API
 * @ingroup     OCKAM
 * @brief       OCKAM_MEMORY_API
 *
 * @addtogroup  OCKAM_MEMORY
 * @{
 ********************************************************************************************************
 */

/*
 ********************************************************************************************************
 *                                             INCLUDE FILES                                            *
 ********************************************************************************************************
 */

#include <stddef.h>
#include <stdint.h>

#include "ockam/error.h"

/*
 ********************************************************************************************************
 *                                                DEFINES                                               *
 ********************************************************************************************************
 */

#define kMemoryErrorInvalidParam (kOckamErrorInterfaceMemory | 0x01)
#define kMemoryErrorInvalidSize (kOckamErrorInterfaceMemory | 0x02)
#define kMemoryErrorAllocFail (kOckamErrorInterfaceMemory | 0x03)

/*
 ********************************************************************************************************
 *                                               DATA TYPES                                             *
 ********************************************************************************************************
 */

typedef OckamError MemoryError;

/**
 *******************************************************************************
 * @struct  OckamMemory
 * @brief   The Ockam Memory interface
 *******************************************************************************
 */

typedef struct {
  /**
   ****************************************************************************************************
   *                                          Create()
   * @brief   Create an ockam memory module.
   *
   * @param   p_arg[in] Configuration structure.
   *
   * @return  kOckamErrorNone on success.
   ****************************************************************************************************
   */

  MemoryError (*Create)(void *p_arg);

  /**
   ****************************************************************************************************
   *                                          Alloc()
   * @brief   Allocate memory from the specified memory module
   *
   * @param   p_buf[in,out] Pointer to the buffer to allocate data for.
   *
   * @param   size[in]      Size of a buffer to allocate.
   *
   * @return  kOckamErrNone on success.
   ****************************************************************************************************
   */

  MemoryError (*Alloc)(void **p_buf, size_t size);

  /**
   ****************************************************************************************************
   *                                          Free()
   * @brief   Free the specified buffer.
   *
   * @param   p_buf[in] Pointer to the buffer to free. Must have been allocated from Alloc().
   *
   * @param   size[in]  Size of the buffer that was allocated. Must match what was specified in Alloc.
   *
   * @return  kOckamErrNone on success.
   ****************************************************************************************************
   */

  MemoryError (*Free)(void *p_buf, size_t size);

  /**
   ****************************************************************************************************
   *                                          Copy()
   * @brief   Copy data from the source buffer to the destination buffer
   *
   * @param   p_dst[out]  Buffer to place the copied data into.
   *
   * @param   p_src[in]   Buffer to copy data from.
   *
   * @param   size[in]    Size of data to copy.
   *
   * @return kOckamErrorNone on success.
   ****************************************************************************************************
   */

  MemoryError (*Copy)(void *p_dst, void *p_src, size_t size);

  /**
   ****************************************************************************************************
   *                                          Set()
   * @brief   Set size number of bytes to p_mem with val
   *
   * @param   p_mem[out]  The buffer to fill with the specified value.
   *
   * @param   val[in]     The value to set to the buffer.
   *
   * @param   size[in]    The number of bytes of val to set to p_mem.
   *
   * @return kOckamErrorNone on success.
   ****************************************************************************************************
   */

  MemoryError (*Set)(void *p_mem, uint8_t val, size_t size);

  /**
   ****************************************************************************************************
   *                                         Move()
   * @brief   Copy num bytes from the source to the destination
   *
   * @param   p_dest[out] The buffer to fill with data from p_src.
   *
   * @param   p_src[in]   The buffer move data to p_dest from.
   *
   * @param   num[in]     The number of bytes to move.
   *
   * @return kOckamErrorNone on success.
   ****************************************************************************************************
   */

  MemoryError (*Move)(void *p_dest, void *p_src, size_t num);

} OckamMemory;

/*
 ********************************************************************************************************
 *                                                EXTERNS                                               *
 ********************************************************************************************************
 */

extern const OckamMemory ockam_memory_stdlib;

/*
 ********************************************************************************************************
 * @}
 ********************************************************************************************************
 */

#endif
