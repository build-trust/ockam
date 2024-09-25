const axios = require('axios');
const { JWT } = require('google-auth-library');
const { BigQuery } = require('@google-cloud/bigquery');

const projectId = process.env.GOOGLE_CLOUD_PROJECT;
if (!projectId) {
    console.error('GOOGLE_CLOUD_PROJECT environment variable must be set.');
    process.exit(1);
}

const credentials_base64 = process.env.GOOGLE_APPLICATION_CREDENTIALS_BASE64;
if (!credentials_base64) {
    console.error('GOOGLE_APPLICATION_CREDENTIALS_BASE64 environment variable must be set.');
    process.exit(1);
}

const private_endpoint_name = process.env.PRIVATE_ENDPOINT_NAME;
if (!private_endpoint_name) {
    console.error('PRIVATE_ENDPOINT_NAME environment variable must be set.');
    process.exit(1);
}

const credentials_json = Buffer.from(credentials_base64, 'base64').toString('utf-8');
const credentials = JSON.parse(credentials_json);

// Function to get Bearer token
const getAuthToken = async () => {
    // Create a JWT client using the credentials
    const client = new JWT({
        email: credentials.client_email,
        key: credentials.private_key,
        scopes: ['https://www.googleapis.com/auth/bigquery'],
    });

    // Authorize the client and get the Bearer token
    const token = await client.authorize();
    return token.access_token;
};

// Custom BigQuery Client
class CustomBigQueryClient extends BigQuery {
    constructor(projectID) {
        super();
        this.projectId = projectID;
    }

    async request(reqOpts, callback) {
        try {
            const token = await getAuthToken();
            const url = `http://127.0.0.1:8080/bigquery/v2/projects/${this.projectId}/${reqOpts.uri}`;
            const checkedURl = url.replace(/([^:]\/)\/+/g, "$1");

            // When deleting dataset, body is sent as an object named qs
            const body = reqOpts.json || reqOpts.qs;

            const config = {
                method: reqOpts.method,
                url: checkedURl,
                headers: {
                    ...reqOpts.headers,
                    'Authorization': `Bearer ${token}`,
                    'Content-Type': 'application/json',
                    'Host': `bigquery-${private_endpoint_name}.p.googleapis.com`,
                },
                data: body,
            };

            const response = await axios(config);
            callback(null, response.data, response);
        } catch (error) {
            callback(error, null, null);
        }
    }
}

const bigQueryClient = new CustomBigQueryClient(projectId);

async function createDataset(datasetId) {
    console.log(`Creating Dataset ${datasetId}`);
    const [dataset] = await bigQueryClient.createDataset(datasetId);
    console.log(`Dataset ${dataset.id} created.`);
}

async function createTable(datasetId, tableId) {
    console.log(`Creating Table ${tableId} for dataset ${datasetId}`);

    const schema = [
        { name: 'name', type: 'STRING' },
        { name: 'age', type: 'INTEGER' },
        { name: 'email', type: 'STRING' }
    ];

    const options = {
        schema: schema
    };

    const [table] = await bigQueryClient
        .dataset(datasetId)
        .createTable(tableId, options);

    console.log(`Table ${table.id} created.`);
}

async function insertData(datasetId, tableId) {
    console.log(`Inserting data to Table ${tableId} for dataset ${datasetId}`);

    const rows = [
        { name: 'John Doe', age: 30, email: 'john.doe@example.com' },
        { name: 'Jane Smith', age: 25, email: 'jane.smith@example.com' }
    ];

    await bigQueryClient
        .dataset(datasetId)
        .table(tableId)
        .insert(rows);

    console.log(`Inserted ${rows.length} rows into ${tableId}`);
}

async function queryData(datasetId, tableId) {
    console.log(`Querying data for Table ${tableId} for dataset ${datasetId}`);

    const query = `
        SELECT name, age, email
        FROM \`${bigQueryClient.projectId}.${datasetId}.${tableId}\`
        WHERE age > 20
    `;

    const [rows] = await bigQueryClient.query({ query });

    console.log('Query Results:');
    rows.forEach(row => {
        console.log(`Name: ${row.name}, Age: ${row.age}, Email: ${row.email}`);
    });
}

async function deleteDataset(datasetId) {
    console.log(`Deleting dataset ${datasetId}`);
    await bigQueryClient.dataset(datasetId).delete({ force: true }); // force: true deletes all tables within the dataset
    console.log(`Dataset ${datasetId} deleted.`);
}

// Run the example
(async () => {
    let datasetId = "ockam_" + (Math.random() + 1).toString(36).substring(7);
    const tableId = 'ockam_table';

    try {
        await createDataset(datasetId);
        await createTable(datasetId, tableId);
        await insertData(datasetId, tableId);
        await queryData(datasetId, tableId);

        console.log(
            "\nThe example run was successful 🥳.\n" +
            "\nThe app connected with bigquery over an encrypted portal." +
            "\nInserted some data, and querried it back.\n",
        );
    } catch (error) {
        console.error('Error:', error);
    }

    await deleteDataset(datasetId);
})();
