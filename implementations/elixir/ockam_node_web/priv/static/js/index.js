(() => {
  class Socket {
    setup() {
      this.socket = new WebSocket("ws://localhost:4000/ws/")

      this.socket.addEventListener("message", (event) => {
        const p = document.createElement("p")
        p.innerHTML = event.data

        document.getElementById("main").append(p)
      })

      this.socket.addEventListener("close", () => {
        this.setupSocket()
      })
    }

    send(event) {
      event.preventDefault()

      this.socket.send(
        JSON.stringify({a: "100"})
      )
    }
  }

  const socket = new Socket()
  socket.setup()

  document.getElementById("button")
    .addEventListener("click", (event) => socket.send(event))
})()
