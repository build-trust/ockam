async function connectAndQuery(query, attempts = 100, waitTimeBetweenAttempts = 10000) {
  // Try to connect
  while (attempts-- > 0) {
    try {
      // Check connection status
      const statusResponse = await fetch("http://ockam:13000/status");
      if (statusResponse.ok) {
        console.log("Connected successfully:", await statusResponse.text());

        // Proceed with query after successful connection
        const queryUrl = "http://ockam:13000/query";
        const options = {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
          },
          body: JSON.stringify({ query }),
        };

        const queryResponse = await fetch(queryUrl, options);
        if (queryResponse.ok) {
          const data = await queryResponse.json();
          console.log("Query result:", data);
          return data; // Return the query result
        } else {
          console.log("Failed to query, status:", queryResponse.status);
          return null; // Return null to indicate the query failed
        }
      } else {
        console.log("Failed to connect, status:", statusResponse.status);
      }
    } catch (error) {
      console.log("Error:", error.message);
    }

    // Wait before the next attempt
    await new Promise((resolve) => setTimeout(resolve, waitTimeBetweenAttempts));
  }

  console.log("All attempts failed.");
  throw new Error("All attempts to connect and query failed");
}

// Example usage
connectAndQuery("Write a funny poem about computer networks")
  .then((data) => {
    if (data) {
      console.log(data);
    }
  })
  .catch((error) => {
    console.log("Operation failed:", error.message);
  });
