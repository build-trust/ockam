package main

import (
	"fmt"
	"github.com/ockam-network/did"
	"github.com/ockam-network/ockam"
	"github.com/ockam-network/ockam/node"
	ockamHttp "github.com/ockam-network/ockam/node/remote/http"
	"log"
	"net/http"
	"os"
	"strconv"
	"strings"
)

func run() error {
	mux := http.NewServeMux()

	port := os.Getenv("PORT")
	discoverer_name := os.Getenv("DISCOVERER_NAME")
	discoverer_port, err := strconv.Atoi(os.Getenv("DISCOVERER_PORT"))

	if err != nil {
		return err
	}

	getHandler, err := handleGetEntity(discoverer_name, discoverer_port)
	if err != nil {
		return err
	}

	mux.Handle(fmt.Sprintf("/%s/identifiers/ockam/", Version()), getHandler)
	log.Println(fmt.Sprintf("/%s/identifiers/ockam/", Version()))
	log.Println(fmt.Sprintf("Listening on %s\n", port))

	if err := http.ListenAndServe(fmt.Sprintf(":%s", port), mux); err != nil {
		return err
	}

	return nil
}

func handleGetEntity(name string, port int) (http.Handler, error) {
	ockamNode, err := node.New(node.PeerDiscoverer(ockamHttp.Discoverer(name, port)))
	if err != nil {
		return nil, err
	}

	err = ockamNode.Sync()
	if err != nil {
		return nil, err
	}

	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		did_path := strings.Split(r.URL.Path, fmt.Sprintf("/%s/identifiers/ockam/", Version()))

		id, err := did.Parse(did_path[1])
		if err != nil {
			w.WriteHeader(http.StatusNotFound)
			w.Write([]byte("Not Found"))
			return
		}

		// Fetch Entity
		bytes, _, err := ockamNode.FetchEntity(id.String())

		if err != nil {
			w.WriteHeader(http.StatusNotFound)
			w.Write([]byte("Not Found"))
			return
		}

		respondWithJson(w, r, http.StatusOK, bytes)
	}), nil
}

func respondWithJson(w http.ResponseWriter, r *http.Request, code int, payload []byte) {
	w.Header().Set("Content-Type", "application/json-ld")
	w.WriteHeader(code)
	w.Write(payload)
}

func main() {
	log.Fatal(run())
}

func Version() string {
	return ockam.Version()
}
