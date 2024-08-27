const { BigQuery } = require('@google-cloud/bigquery');
const process = require('process');

const projectId = process.env.GOOGLE_CLOUD_PROJECT;
if (!projectId) {
    console.error('GOOGLE_CLOUD_PROJECT environment variable must be set.');
    process.exit(1);
}

const credentials_base64 = process.env.GOOGLE_APPLICATION_CREDENTIALS_BASE64;
const credentials_json = Buffer.from(credentials_base64, 'base64').toString('utf-8');

const credentials = JSON.parse(credentials_json);

// Configure the endpoint to our portal
const bigqueryOptions = {
    projectId: projectId,
    apiEndpoint: 'http://127.0.0.1:8080',
    maxRetries: 100,
    credentials: credentials
};

// Create a BigQuery client
const bigquery = new BigQuery(bigqueryOptions);

async function createDataset(datasetId) {
    const [dataset] = await bigquery.createDataset(datasetId);
    console.log(`Dataset ${dataset.id} created.`);
}

async function createTable(datasetId, tableId) {
    const schema = [
        { name: 'name', type: 'STRING' },
        { name: 'age', type: 'INTEGER' },
        { name: 'email', type: 'STRING' }
    ];

    const options = {
        schema: schema
    };

    const [table] = await bigquery
        .dataset(datasetId)
        .createTable(tableId, options);

    console.log(`Table ${table.id} created.`);
}

async function insertData(datasetId, tableId) {
    const rows = [
        { name: 'John Doe', age: 30, email: 'john.doe@example.com' },
        { name: 'Jane Smith', age: 25, email: 'jane.smith@example.com' }
    ];

    await bigquery
        .dataset(datasetId)
        .table(tableId)
        .insert(rows);

    console.log(`Inserted ${rows.length} rows into ${tableId}`);
}

async function queryData(datasetId, tableId) {
    const query = `
        SELECT name, age, email
        FROM \`${bigquery.projectId}.${datasetId}.${tableId}\`
        WHERE age > 20
    `;

    const [rows] = await bigquery.query({ query });

    console.log('Query Results:');
    rows.forEach(row => {
        console.log(`Name: ${row.name}, Age: ${row.age}, Email: ${row.email}`);
    });
}

async function deleteDataset(datasetId) {
    await bigquery.dataset(datasetId).delete({ force: true }); // force: true deletes all tables within the dataset
    console.log(`Dataset ${datasetId} deleted.`);
}

// Running all steps
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
