const { MongoClient } = require("mongodb");

// Configuration to connect to the MongoDB database on - ockam:17017
//
// ockam:17017 is the inlet address of the tcp portal to MongoDB.
// This inlet is local to analysis_corp, the MongoDB database is remote in bank_corp.
const uri = "mongodb://ockam:17017/mydb"

// Attempt to connect to the database in a loop.
//
// Since all the containers may take a few seconds to start up.
// We'll attempt to connect in a loop once every 10 seconds by default.
async function connect(attempts = 100, waitTimeBetweenAttempts = 10000) {
  while (attempts--) {
    const client = new MongoClient(uri);
    try {
      await client.connect();
      console.log("Connected to the database.\n");
      return client;
    } catch (err) {
      console.log(`Couldn't connect to the database: ${err.message}, retrying ...`);
      if (attempts == 0) throw err;
      await new Promise((resolve) => setTimeout(resolve, waitTimeBetweenAttempts));
    }
  }
}

async function run() {
  const client = await connect();
  try {
    const database = client.db('sample');
    const users = database.collection('users');

    console.log("Inserting some data into the users collections ...");
    const docs = ["Alice", "Bob", "Charlie"].map((name) => ({ name: name, score: Math.floor(Math.random() * 101) }));
    await users.insertMany(docs);

    const result = await users.find().toArray()
    console.log("USERS:", result);
    console.log(
      "\nThe example run was successful 🥳.\n" +
      "\nThe app connected with the database through an encrypted portal." +
      "\nInserted some data, and querried it back.\n",
    );
  } finally {
    await client.close();
    console.log("Disconnected from the database.");
  }

}
run()
