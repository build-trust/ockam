#include "errlog.h"

FILE* g_err_log;

void init_err_log(FILE* fp) {
	if(NULL == fp) {
		g_err_log = stdout;
	} else {
		g_err_log = fp;
	}
}

void log_error(char* message) {
	fprintf(g_err_log, "%s\n", message);
}
