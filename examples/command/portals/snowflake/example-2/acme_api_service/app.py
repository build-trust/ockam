from flask import Flask
import random

app = Flask(__name__)

responses = {
    "greetings": ["Hello", "Hi", "Hey", "Greetings", "Welcome", "Bonjour", "Hola"],
    "farewells": ["Goodbye", "See you", "Farewell", "Bye", "Adios"]
}

@app.route("/")
def hello_world():
    return "<p>Hello!</p>"


@app.route("/ping")
def ping():
    return "pong"


@app.route("/greet")
def greet():
    greeting = random.choice(responses["greetings"])
    return f"<p>{greeting}!</p>"


@app.route("/farewell")
def farewell():
    farewell = random.choice(responses["farewells"])
    return f"<p>{farewell}!</p>"


if __name__ == "__main__":
    app.run(host='0.0.0.0', port=15000)
