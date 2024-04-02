from flask import Flask

app = Flask(__name__)


@app.route("/")
def hello_world():
    return "<p>Hello!</p>"


@app.route("/ping")
def ping():
    return "pong"
