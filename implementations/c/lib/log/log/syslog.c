#include "ockam/syslog.h"

#include <stdio.h>

#include "ockam/error.h"
FILE* g_err_log = NULL;

void init_err_log(FILE* fp) {
  if (NULL == fp) {
    g_err_log = stdout;
  } else {
    g_err_log = fp;
  }
}

void log_error(ockam_error_t error, const char* message) {
  if (NULL == g_err_log) g_err_log = stdout;
  fprintf(g_err_log, "Error %d %.8x: %s\n", error, error, message);
}
