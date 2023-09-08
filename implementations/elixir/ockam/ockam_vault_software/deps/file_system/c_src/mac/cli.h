#ifndef CLI_H
#define CLI_H

#include "common.h"

#ifndef CLI_NAME
#define CLI_NAME "fsevent_watch"
#endif /* CLI_NAME */

struct cli_info {
  UInt64 since_when_arg;
  double latency_arg;
  bool no_defer_flag;
  bool watch_root_flag;
  bool ignore_self_flag;
  bool file_events_flag;
  bool mark_self_flag;
  int format_arg;

  char** inputs;
  unsigned inputs_num;
};

extern const char* cli_info_purpose;
extern const char* cli_info_usage;
extern const char* cli_info_help[];

void cli_print_help(void);
void cli_print_version(void);

int cli_parser (int argc, const char** argv, struct cli_info* args_info);
void cli_parser_init (struct cli_info* args_info);
void cli_parser_free (struct cli_info* args_info);


#endif /* CLI_H */
