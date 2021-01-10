#include <stddef.h>
#include <ockam/log.h>

#if OCKAM_LOG_ENABLED
#if OCKAM_CUSTOM_LOG_FUNCTION
static ockam_log_function_t ockam_log_function;
void ockam_set_log_function(ockam_log_function_t log_function) {
    ockam_log_function = log_function;
}
#else
#include <stdio.h>
#include <time.h>

static const char *level_strings[] = {
        "OCKAM_INFO",
        "OCKAM_DEBUG",
        "OCKAM_WARN",
        "OCKAM_ERROR",
        "OCKAM_FATAL",
};

static void ockam_log_printf(ockam_log_level_t level, const char *file, int line, const char *fmt, va_list args) {
    time_t t = time(NULL);

    const struct tm* local_time = localtime(&t);

    char time_str[9];
    strftime(time_str, sizeof(time_str), "%H:%M:%S", local_time);

    fprintf(stdout, "%s %-11s %s:%d: ", time_str, level_strings[level], file, line);
    vfprintf(stdout, fmt, args);
    fprintf(stdout, "\n");

    fflush(stdout);
}
static ockam_log_function_t ockam_log_function = ockam_log_printf;
#endif
#else
static ockam_log_function_t ockam_log_function = NULL;
#endif

// Default log level
static ockam_log_level_t ockam_log_level = OCKAM_LOG_LEVEL_INFO;

void ockam_log_set_level(ockam_log_level_t level) {
    ockam_log_level = level;
}

ockam_log_level_t ockam_log_get_level() {
    return ockam_log_level;
}

void ockam_log_log(ockam_log_level_t level, const char *file, int line, const char *fmt, ...) {
    if (ockam_log_level > level) {
        return;
    }

    if (NULL != ockam_log_function) {
        va_list args;
        va_start(args, fmt);
        ockam_log_function(level, file, line, fmt, args);
        va_end(args);
    }
}
