/**
 * @file  error.h
 * @brief Ockam common error defines
 */

#ifndef OCKAM_ERROR_H_
#define OCKAM_ERROR_H_

#include <stdbool.h>

typedef struct {
  int code;
  const char* domain;
} ockam_error_t;

typedef enum {
  OCKAM_ERROR_NONE = 0,
} ockam_error_code_t;

static inline bool ockam_error_is_none(const ockam_error_t* error) {
  return error->code == OCKAM_ERROR_NONE;
}

static inline bool ockam_error_has_error(const ockam_error_t* error) {
  return !ockam_error_is_none(error);
}

#endif
