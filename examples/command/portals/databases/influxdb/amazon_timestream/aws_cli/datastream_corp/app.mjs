'use strict';

import { InfluxDB, Point, flux } from '@influxdata/influxdb-client';
import os from 'os';
import { execSync } from 'child_process';
import * as https from 'https';

/** $TOKEN and $ORG_ID gets replaced after InfluxDB is created  **/
const url = process.env.INFLUX_URL || 'https://localhost:8086'
const token = process.env.INFLUX_TOKEN || '$TOKEN';
const org = process.env.INFLUX_ORG || '$ORG_ID';
const bucket = process.env.INFLUX_BUCKET || 'ockam_demo_bucket';

// TODO: Remove upon Ockam inlet/outlet handling TLS
const httpsAgent = new https.Agent({
  rejectUnauthorized: false
});

const influxDB = new InfluxDB({ url, token, transportOptions: { agent: httpsAgent } });

const writeApi = influxDB.getWriteApi(org, bucket);

async function writeData() {
  const hostname = os.hostname();
  let cpuLoad;
  let freeDiskSpace;

  try {
    cpuLoad = parseFloat(execSync("uptime | awk '{print $(NF-2)}' | sed 's/,//'").toString().trim());
    freeDiskSpace = parseInt(execSync("df -BG / | tail -n 1 | awk '{print $4}' | sed 's/G//'").toString().trim(), 10);
  } catch (error) {
    console.error('Error extracting system metrics:', error);
    return;
  }

  if (isNaN(cpuLoad) || isNaN(freeDiskSpace)) {
    console.error('Extracted metrics are NaN', { cpuLoad, freeDiskSpace });
    return;
  }

  const point = new Point('system_metrics')
    .tag('host', hostname)
    .floatField('cpu_load', cpuLoad)
    .intField('free_disk_space', freeDiskSpace);

  console.log(`Writing point: ${point.toLineProtocol(writeApi)}`);

  writeApi.writePoint(point);

  await writeApi.close().then(() => {
    console.log('WRITE FINISHED');
  }).catch(e => {
    console.error('Write failed', e);
  });
}

async function queryData() {
  const queryApi = influxDB.getQueryApi(org);
  const query = flux`
    from(bucket: "${bucket}")
    |> range(start: -1h)
    |> filter(fn: (r) => r._measurement == "system_metrics")
  `;

  console.log('Querying data:');

  queryApi.queryRows(query, {
    next(row, tableMeta) {
      const fieldValue = row[5];
      const fieldName = row[6];

      let cpuLoad = 'N/A';
      let freeDiskSpace = 'N/A';

      if (fieldName === 'cpu_load') {
        cpuLoad = fieldValue;
      } else if (fieldName === 'free_disk_space') {
        freeDiskSpace = fieldValue;
      }

      console.log(`cpu_load=${cpuLoad}, free_disk_space=${freeDiskSpace}`);
    },
    error(error) {
      console.error('Query failed', error);
    },
    complete() {
      console.log(
        "\nThe example run was successful 🥳.\n" +
        "\nThe app connected with the database through an encrypted portal." +
        "\nCreated a table, inserted some data, and querried it back.\n"
      );
    },
  });
}

writeData().then(() => {
  setTimeout(() => {
    queryData();
  }, 3000);
});
