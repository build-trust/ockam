#include <stdio.h>
#include "error.h"

void init_err_log(FILE* fp);

void log_error(OCKAM_ERR error, char* message);
