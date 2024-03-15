async function run(attempts = 20, waitTimeBetweenAttempts = 2000) {
  while (attempts--) {
    try {
      console.log(`Connecting ...`);
      const statusResponse = await fetch("http://localhost:3000/status");
      const jsonResponse = await statusResponse.json();

      console.log("Response:", jsonResponse);
      console.log(
        "\nThe example run was successful 🥳.\n" +
          "The app made an API request to the monitoring API over an encrypted portal and got back a response.\n",
      );

      return;
    } catch (error) {
      console.log(error);
      if (attempts > 0) {
        console.log(`Waiting for ${waitTimeBetweenAttempts / 1000} seconds before attempting again...`);
        await new Promise((resolve) => setTimeout(resolve, waitTimeBetweenAttempts));
      }
    }
  }

  console.log("All attempts failed.");
  return;
}

run();
