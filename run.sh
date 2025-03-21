target/release/ockam reset -y

OCKAM_LOGGING=true OCKAM_LOG_LEVEL=info target/release/ockam node create -vv --foreground "
name: test
tcp-outlets:
  regular_outlet:
    to: 127.0.0.1:6666
  psql_outlet1:
    to: 127.0.0.1:6543
    psql-tls: true
  psql_outlet2:
    to: 127.0.0.1:6544
    psql-tls: true
  python_outlet:
    to: 127.0.0.1:6667
tcp-inlets:
  regular_inlet:
    from: 5433
    to: /secure/api/service/regular_outlet
  root:
    from: 5432
  psql_inlet1:
    to: /secure/api/service/psql_outlet1
    sni: example1.com
  psql_inlet2:
    to: /secure/api/service/psql_outlet2
    sni: example2.com
  python_inlet:
    to: /secure/api/service/python_outlet
    sni: example3.com
"
