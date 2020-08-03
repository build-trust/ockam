#ifndef OCKAM_LOG_H
#define OCKAM_LOG_H

#include <stdarg.h>

#ifdef OCKAM_DISABLE_LOG
#define OCKAM_LOG_ENABLED 0
#else
#define OCKAM_LOG_ENABLED 1
#endif

typedef enum {
    OCKAM_LOG_LEVEL_INFO = 0,
    OCKAM_LOG_LEVEL_DEBUG,
    OCKAM_LOG_LEVEL_WARN,
    OCKAM_LOG_LEVEL_ERROR,
    OCKAM_LOG_LEVEL_FATAL,
} ockam_log_level_t;

typedef void (*ockam_log_function_t)(ockam_log_level_t level, const char *file, int line, const char *fmt, va_list args);

#if OCKAM_CUSTOM_LOG_FUNCTION
void ockam_set_log_function(ockam_log_function_t log_function);
#endif

void ockam_log_set_level(ockam_log_level_t level);
ockam_log_level_t ockam_log_get_level();

void ockam_log_log(ockam_log_level_t level, const char *file, int line, const char *fmt, ...);

#define ockam_log_info(...) \
        do { if (OCKAM_LOG_ENABLED) ockam_log_log(OCKAM_LOG_LEVEL_INFO, __FILE__, __LINE__, __VA_ARGS__); } while(0)

#define ockam_log_debug(...) \
        do { if (OCKAM_LOG_ENABLED) ockam_log_log(OCKAM_LOG_LEVEL_DEBUG, __FILE__, __LINE__, __VA_ARGS__); } while(0)

#define ockam_log_warn(...) \
        do { if (OCKAM_LOG_ENABLED) ockam_log_log(OCKAM_LOG_LEVEL_WARN, __FILE__, __LINE__, __VA_ARGS__); } while(0)

#define ockam_log_error(...) \
        do { if (OCKAM_LOG_ENABLED) ockam_log_log(OCKAM_LOG_LEVEL_ERROR, __FILE__, __LINE__, __VA_ARGS__); } while(0)

#define ockam_log_fatal(...) \
        do { if (OCKAM_LOG_ENABLED) ockam_log_log(OCKAM_LOG_LEVEL_FATAL, __FILE__, __LINE__, __VA_ARGS__); } while(0)

#endif //OCKAM_LOG_H
