import os
from openai import AzureOpenAI

# Initialize Azure OpenAI client
client = AzureOpenAI(
    azure_endpoint=os.getenv('AZURE_OPENAI_ENDPOINT'),
    api_key=os.getenv('AZURE_OPENAI_API_KEY'),
    api_version="2024-02-15-preview"
)

try:
    print(f"Connecting to: {client.base_url}")

    response = client.chat.completions.create(
        model="gpt-4o-mini",
        messages=[
            {"role": "user", "content": "What is Ockham's Razor?"}
        ]
    )

    print("\nResponse:", response.choices[0].message.content)
    print("\nThe example run was successful 🥳.")

except Exception as e:
    print(f"An error occurred: {e}")
