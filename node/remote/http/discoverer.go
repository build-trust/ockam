package http

import (
	"net"

	"github.com/ockam-network/ockam"
	"github.com/pkg/errors"
)

type dnsDiscoverer struct {
	name string
	port int
}

// Discoverer returns
func Discoverer(name string, port int) ockam.NodeDiscoverer {
	return &dnsDiscoverer{name: name, port: port}
}

// Discover is
func (d *dnsDiscoverer) Discover() ([]ockam.Node, error) {
	var nodes []ockam.Node

	ips, err := net.LookupIP(d.name)
	if err != nil {
		return nil, errors.WithStack(err)
	}

	for _, ip := range ips {
		n, err := NewNode(IP(ip.String()), Port(d.port))
		if err != nil {
			return nil, errors.WithStack(err)
		}
		nodes = append(nodes, n)
	}

	return nodes, nil
}
