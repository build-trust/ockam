#include <stdio.h>
#include <stdlib.h>

#include "ockam/error.h"
#include "ockam/syslog.h"
#include "ockam/transport.h"

TransportError file_compare(char *p_f1, char *p_f2) {
  TransportError status = 0;

  unsigned more = 1;

  FILE *fp1 = NULL;
  FILE *fp2 = NULL;

  char buffer1[256];
  char buffer2[256];

  size_t r1;
  size_t r2;

  fp1 = fopen(p_f1, "r");
  fp2 = fopen(p_f2, "r");

  if ((NULL == fp1) || (NULL == fp2)) {
    status = kTestFailure;
    goto exit_block;
  }

  while (more) {
    r1 = fread(buffer1, 1, sizeof(buffer1), fp1);
    r2 = fread(buffer2, 1, sizeof(buffer2), fp2);
    if (r1 != r2) {
      status = kTestFailure;
      goto exit_block;
    }
    if (0 != memcmp(buffer1, buffer2, r1)) {
      status = kTestFailure;
      goto exit_block;
    }
    if (feof(fp1)) {
      if (!feof(fp2)) {
        status = kTestFailure;
        goto exit_block;
      }
      more = 0;
    }
  }

exit_block:
  if (NULL != fp1) fclose(fp1);
  if (NULL != fp2) fclose(fp2);
  return status;
}
