This tool can be used to stress test a project and see how it behaves under heavy load.

It shares the same `OCKAM_HOME` of the `ockam` command, and you need to perform the
set up using the `ockam` command before using it.
A setup could be either an `ockam enroll` or a `ockam project import`.
Relays are created using the same code as `ockam` command but portals are created using
a simpler implementation which doesn't involve TCP sockets.

You can also use the `deploy` script in this directory to deploy the stress test to a remote machine.

```
Usage: stress-test <COMMAND>

Commands:
run       Run the stress test
validate  Validate the configuration file
generate  Generate sample configuration files
help      Print this message or the help of the given subcommand(s)

Options:
-h, --help  Print help
```

To debug failures, you can use:

```
OCKAM_LOG_LEVEL=error stress-test run <config.toml> --log
```

Sample configuration:

```
peak_portals = 20
peak_relays = 10
ramp_up = 60
throughput = "1 mbits"
project = "/project/default"
```

| Parameter    | Default          | Description                                            | Possible values                                                                          |
|--------------|------------------|--------------------------------------------------------|------------------------------------------------------------------------------------------|
| peak_portals | 0                | Number of portals to create                            | positive integers, 1_000 is allowed                                                      |
| peak_relays  | 0                | Number of relays to create, at least 1 is created      | positive integers, 1_000 is allowed                                                      |
| ramp_up      | 0                | Time, in seconds, to create all the portals and relays | positive integers                                                                        |
| throughput   | unlimited        | Throughput to use for each portal                      | unlimited or positive integer followed by GBits,Mbits,Kbits,Bits, 1_000 Mbits is allowed |
| project      | /project/default | Route to any project to test                           | Any route as long as it reaches a project                                                |

Sample output:

```
|  Elapsed  | Portals | Relays | M. sent | M. recv | In-fli |  B. sent  |  B. recv  | Spe. sent  | Spe. recv  | M. OOO | Errors |
|    00s    |    0    |   0    |    0    |    0    |   0    |    0 B    |    0 B    |  0.00 bps  |  0.00 bps  |   0    |   0    |
|    01s    |    0    |   0    |    0    |    0    |   0    |    0 B    |    0 B    |  0.00 bps  |  0.00 bps  |   0    |   0    |
|    02s    |    0    |   1    |    0    |    0    |   0    |    0 B    |    0 B    |  0.00 bps  |  0.00 bps  |   0    |   0    |
|    03s    |    1    |   1    |    1    |    1    |   0    | 12.21 KB  | 12.21 KB  | 25.00 Kbps | 25.00 Kbps |   0    |   0    |
|    04s    |    1    |   1    |    2    |    2    |   0    | 24.41 KB  | 24.41 KB  | 40.00 Kbps | 40.00 Kbps |   0    |   0    |
|    05s    |    7    |   5    |    9    |    7    |   2    |  0.11 MB  | 85.45 KB  | 0.15 Mbps  | 0.12 Mbps  |   0    |   0    |
|    06s    |    7    |   5    |   16    |   16    |   0    |  0.19 MB  |  0.19 MB  | 0.23 Mbps  | 0.23 Mbps  |   0    |   0    |
|    07s    |   11    |   9    |   27    |   23    |   4    |  0.32 MB  |  0.27 MB  | 0.34 Mbps  | 0.29 Mbps  |   0    |   0    |
|    08s    |   11    |   9    |   38    |   38    |   0    |  0.45 MB  |  0.45 MB  | 0.42 Mbps  | 0.42 Mbps  |   0    |   0    |
|    09s    |   16    |   12   |   54    |   49    |   5    |  0.64 MB  |  0.58 MB  | 0.54 Mbps  | 0.49 Mbps  |   0    |   0    |
|  Elapsed  | Portals | Relays | M. sent | M. recv | In-fli |  B. sent  |  B. recv  | Spe. sent  | Spe. recv  | M. OOO | Errors |
|    10s    |   16    |   12   |   68    |   66    |   2    |  0.81 MB  |  0.79 MB  | 0.68 Mbps  | 0.66 Mbps  |   0    |   0    |
|    11s    |   16    |   15   |   86    |   82    |   4    |  1.03 MB  |  0.98 MB  | 0.86 Mbps  | 0.82 Mbps  |   0    |   0    |
|    12s    |   20    |   15   |   107   |   101   |   6    |  1.28 MB  |  1.20 MB  | 1.07 Mbps  | 1.01 Mbps  |   0    |   0    |
|    13s    |   20    |   15   |   127   |   121   |   6    |  1.51 MB  |  1.44 MB  | 1.26 Mbps  | 1.20 Mbps  |   0    |   0    |
|    14s    |   20    |   15   |   143   |   140   |   3    |  1.70 MB  |  1.67 MB  | 1.41 Mbps  | 1.38 Mbps  |   0    |   0    |
```

| Column    | Description                                                                                 |
|-----------|---------------------------------------------------------------------------------------------|
| Elapsed   | Time elapsed since the start of the test                                                    |
| Portals   | Number of portals created                                                                   |
| Relays    | Number of relays created                                                                    |
| M. sent   | Messages sent                                                                               |
| M. recv   | Messages received                                                                           |
| In-fli    | Messages in flight, sent but not yet received                                               |
| B. sent   | Total amount of bytes sent                                                                  |
| B. recv   | Total amount of bytes received                                                              |
| Spe. sent | Avarage outgoing speed of the last 10 seconds, in bits per second                           |
| Spe. recv | Avarage incoming speed of the last 10 seconds, in bits per second                           |
| M. OOO    | Messages out of order, it can also detect a lost packet if the following packet is received |
| Errors    | Number of errors during relay or portal creation                                            |
