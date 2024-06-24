const { Client } = require("pg");

// Configuration to connect to the postgres database on - localhost:15432
//
// localhost:15432 is the inlet address of the tcp portal to postgres.
// This inlet is local to analysis_corp, the postgres database is remote in bank_corp.
const clientConfig = { host: "localhost", port: 15432, user: "postgres", password: "postgres", database: "test" };

// Attempt to connect to the database in a loop.
//
// Since all the containers may take a few seconds to start up.
// We'll attempt to connect in a loop once every 10 seconds by default.
async function connect(attempts = 100, waitTimeBetweenAttempts = 10000) {
  while (attempts--) {
    const client = new Client(clientConfig);
    try {
      await client.connect();
      console.log("Connected to the database.\n");
      return client;
    } catch (err) {
      console.log(`Couldn't connect to the database: ${err.message}, retrying ...`);
      await client.end();
      if (attempts == 0) throw err;
      await new Promise((resolve) => setTimeout(resolve, waitTimeBetweenAttempts));
    }
  }
}

// This function is the core of our example app.
//
// We connect to the database through the inlet.
// Create a new table called users, if it doesn't already exist.
// We then insert some data into the users table and query it back.
//
// Finally we print that our app, running in analysis_corp was able
// to successfully connect and use the postgres database in bank_corp
// through an encrypted portal.
async function run() {
  const client = await connect();
  try {
    console.log("Creating users table ...");
    await client.query(`CREATE TABLE IF NOT EXISTS users (name VARCHAR(255), score INTEGER)`);

    console.log("Inserting some data into the users table ...");
    const users = ["Alice", "Bob", "Charlie"].map((name) => ({ name, score: Math.floor(Math.random() * 101) }));
    for (const user of users) {
      await client.query(`INSERT INTO users (name, score) VALUES ($1, $2)`, [user.name, user.score]);
    }

    console.log("Querying the users table ...");
    const res = await client.query(`SELECT * FROM users`);
    console.log("USERS:", res.rows);

    console.log(
      "\nThe example run was successful 🥳.\n" +
      "\nThe app connected with the database through an encrypted portal." +
      "\nCreated a table, inserted some data, and querried it back.\n",
    );
  } catch (err) {
    console.error("Error:", err.message);
  } finally {
    await client.end();
    console.log("Disconnected from the database.");
  }
}

run();
