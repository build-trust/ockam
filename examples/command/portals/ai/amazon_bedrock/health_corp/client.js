async function run() {
    try {
        const statusResponse = await fetch("http://127.0.0.1:3000/status");
        if (!statusResponse.ok) {
            console.log("Connection failed, status:", statusResponse.status);
            return;
        }

        const query = "What is Ockham's Razor?";
        console.log("Connected to the model.\n\nApp: ", query);
        const queryResponse = await fetch("http://127.0.0.1:3000/query", {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ query }),
            keepalive: true,
        });

        if (!queryResponse.ok) {
            console.log("Query failed, status:", queryResponse.status);
            console.log("Error:", queryResponse.error);
            return;
        }

        const answer = await queryResponse.json();
        console.log(answer['answer']);
        console.log("\nThe example run was successful 🥳.");
    } catch (error) {
        console.log("Error:", error);
    }
}

run();
