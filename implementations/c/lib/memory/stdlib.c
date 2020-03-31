/**
 ********************************************************************************************************
 * @file    stdlib.c
 * @brief   Implementation of Ockam's memory functions using stdlib calls
 ********************************************************************************************************
 */

/*
 ********************************************************************************************************
 *                                             INCLUDE FILES                                            *
 ********************************************************************************************************
 */

#include <ockam/memory.h>
#include <stdlib.h>
#include <string.h>

/*
 ********************************************************************************************************
 *                                                DEFINES                                               *
 ********************************************************************************************************
 */

/*
 ********************************************************************************************************
 *                                               CONSTANTS                                              *
 ********************************************************************************************************
 */

/*
 ********************************************************************************************************
 *                                               DATA TYPES                                             *
 ********************************************************************************************************
 */

/*
 ********************************************************************************************************
 *                                          FUNCTION PROTOTYPES                                         *
 ********************************************************************************************************
 */

MemoryError MemoryStdlibCreate(void *p_arg);
MemoryError MemoryStdlibAlloc(void **p_buf, size_t size);
MemoryError MemoryStdlibFree(void *p_buf, size_t size);
MemoryError MemoryStdlibCopy(void *p_dest, void *p_src, size_t size);
MemoryError MemoryStdlibSet(void *p_mem, uint8_t val, size_t size);
MemoryError MemoryStdlibMove(void *p_dest, void *p_src, size_t num);

/*
 ********************************************************************************************************
 *                                            GLOBAL VARIABLES                                          *
 ********************************************************************************************************
 */

const OckamMemory ockam_memory_stdlib = {&MemoryStdlibCreate, &MemoryStdlibAlloc, &MemoryStdlibFree,
                                         &MemoryStdlibCopy,   &MemoryStdlibSet,   &MemoryStdlibMove};

/*
 ********************************************************************************************************
 *                                           GLOBAL FUNCTIONS                                           *
 ********************************************************************************************************
 */

/*
 ********************************************************************************************************
 *                                            LOCAL FUNCTIONS                                           *
 ********************************************************************************************************
 */

/**
 ********************************************************************************************************
 *                                             MemoryStdlibCreate()
 ********************************************************************************************************
 */

MemoryError MemoryStdlibCreate(void *p_arg) {
  MemoryError ret_val = kOckamErrorNone;

  return ret_val;
}

/**
 ********************************************************************************************************
 *                                             stdlib_alloc()
 ********************************************************************************************************
 */

MemoryError MemoryStdlibAlloc(void **p_buf, size_t size) {
  MemoryError ret_val = kOckamErrorNone;

  if (size == 0) {
    ret_val = kMemoryErrorInvalidSize;
    goto exit_block;
  }

  *p_buf = malloc(size);

  if (*p_buf == 0) {
    ret_val = kMemoryErrorAllocFail;
    goto exit_block;
  }

  memset(*p_buf, 0, size);

exit_block:
  return ret_val;
}

/**
 ********************************************************************************************************
 *                                           stdlib_free()
 ********************************************************************************************************
 */

MemoryError MemoryStdlibFree(void *p_buf, size_t size) {
  MemoryError ret_val = kOckamErrorNone;

  (void)size;

  if (p_buf == 0) {
    ret_val = kMemoryErrorInvalidParam;
    goto exit_block;
  }

  free(p_buf);

exit_block:
  return ret_val;
}

/**
 ********************************************************************************************************
 *                                           stdlib_copy()
 ********************************************************************************************************
 */

MemoryError MemoryStdlibCopy(void *p_dest, void *p_src, size_t size) {
  MemoryError ret_val = kOckamErrorNone;

  if ((p_dest == 0) || (p_src == 0)) {
    ret_val = kMemoryErrorInvalidParam;
    goto exit_block;
  }

  memcpy(p_dest, p_src, size);

exit_block:
  return ret_val;
}

/**
 ********************************************************************************************************
 *                                            stdlib_set()
 ********************************************************************************************************
 */

MemoryError MemoryStdlibSet(void *p_mem, uint8_t val, size_t size) {
  MemoryError ret_val = kOckamErrorNone;

  if (p_mem == 0) {
    ret_val = kMemoryErrorInvalidParam;
    goto exit_block;
  }

  memset(p_mem, val, size);

exit_block:
  return ret_val;
}

/**
 ********************************************************************************************************
 *                                        MemoryStdlibMove()
 ********************************************************************************************************
 */

MemoryError MemoryStdlibMove(void *p_dest, void *p_src, size_t num) {
  MemoryError ret_val = kOckamErrorNone;

  if ((p_src == 0) || (p_dest == 0)) {
    ret_val = kMemoryErrorInvalidParam;
    goto exit_block;
  }

  memmove(p_dest, p_src, num);

exit_block:
  return ret_val;
}
