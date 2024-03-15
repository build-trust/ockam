async function run() {
  try {
    const statusResponse = await fetch("http://localhost:3000/status");
    if (!statusResponse.ok) {
      console.log("Connection failed, status:", statusResponse.status);
      return;
    }

    const query = "Write a 3 line poem about computers";
    const queryResponse = await fetch("http://localhost:3000/query", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ query }),
    });

    if (!queryResponse.ok) {
      console.log("Query failed, status:", queryResponse.status);
      return;
    }

    const answer = await queryResponse.json();
    console.log("Model: ", answer);
  } catch (error) {
    console.log("Error:", error.message);
  }
}

run();
