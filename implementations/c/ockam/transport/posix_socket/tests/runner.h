#ifndef OCKAM_RUNNER_H
#define OCKAM_RUNNER_H

enum TransportType { TCP, UDP };

int run(enum TransportType transport_type, int argc, char* argv[]);

#endif // OCKAM_RUNNER_H
