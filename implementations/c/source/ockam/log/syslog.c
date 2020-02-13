#include <stdlib.h>
#include <stdio.h>
#include "ockam/syslog.h"

FILE* g_err_log = NULL;

void init_err_log(FILE* fp) {
    if(NULL == fp) {
    g_err_log = stdout;
    } else {
    g_err_log = fp;
    }
}

void log_error(OCKAM_ERR error, char* message) {
    if( NULL == g_err_log ) g_err_log = stdout;
    fprintf(g_err_log, "Error %d: %s\n", error, message);
}
