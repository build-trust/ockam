import {
    BedrockRuntimeClient,
    InvokeModelCommand,
} from "@aws-sdk/client-bedrock-runtime";

import {AWS_REGION, MODEL_ID} from './constants.mjs';

async function callModel() {
    const client = new BedrockRuntimeClient({region: AWS_REGION});

    const params = {
        inputText: "is anybody here?",
        textGenerationConfig: {
            maxTokenCount: 4096,
            stopSequences: [],
            temperature: 0,
            topP: 1
        },
    };

    const command = new InvokeModelCommand({
        contentType: "application/json",
        accept: "application/json",
        body: JSON.stringify(params),
        modelId: MODEL_ID,
    });
    try {
        await client.send(command);
    } catch (error) {
        console.log(error.message)
        process.exit(1)
    }
}

callModel();
