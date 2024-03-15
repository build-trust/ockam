const express = require("express");
const app = express();
const port = 3000;

app.get("/status", (req, res) => {
  res.json({ status: "running" });
});

app.listen(port, () => {
  console.log(`The API is listening on port ${port}`);
});
