import express from "express";
import {fileURLToPath} from "url";
import {
    BedrockRuntimeClient,
    InvokeModelCommand,
} from "@aws-sdk/client-bedrock-runtime";

const app = express();
app.use(express.json());

const AWS_REGION = "us-east-1";
const MODEL_ID = "amazon.titan-text-lite-v1";

app.get("/status", (req, res) => {
    res.json({status: "running"});
});

app.post("/query", async (req, res) => {
    try {
        const {query} = req.body;
        if (!query) {
            return res.status(400).json({error: "No query provided"});
        }

        // Create a new Bedrock Runtime client instance.
        const client = new BedrockRuntimeClient({region: AWS_REGION});

        // Prepare the payload for the model.
        const payload = {
            inputText: query,
            textGenerationConfig: {
                maxTokenCount: 4096,
                stopSequences: [],
                temperature: 0,
                topP: 1
            },
        };

        // Invoke the model with the payload and wait for the response.
        const apiResponse = await client.send(
            new InvokeModelCommand({
                contentType: "application/json",
                accept: "application/json",
                body: JSON.stringify(payload),
                modelId: MODEL_ID,
            }),
        );

        // Decode and return the response(s)
        const decodedResponseBody = new TextDecoder().decode(apiResponse.body);
        const responseBody = JSON.parse(decodedResponseBody);
        const responses = responseBody.results;

        if (responses.length >= 1) {
            const answer = responses[0].outputText;
            res.json({query, answer});
        } else {
            const answer = "there was no response to the query";
            res.json({query, answer});
        }
    } catch (error) {
        console.error(error);
        res.status(500).json({error});
    }
});

const PORT = process.env.PORT || 3000;
app.listen(PORT, () => console.log(`Server is running on http://localhost:${PORT}`));
