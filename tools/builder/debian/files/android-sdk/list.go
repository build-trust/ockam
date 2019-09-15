package main

import (
	"encoding/xml"
	"fmt"
	"io/ioutil"
	"log"
	"net/http"
)

type Sdk struct {
	Licenses       []License       `xml:"license"`
	Channels       []Channel       `xml:"channel"`
	RemotePackages []RemotePackage `xml:"remotePackage"`
}

type License struct {
	Id   string `xml:"id,attr"`
	Type string `xml:"type,attr"`
	Text string `xml:",chardata"`
}

type Channel struct {
	Id   string `xml:"id,attr"`
	Name string `xml:",chardata"`
}

type RemotePackage struct {
	Path         string       `xml:"path,attr"`
	DisplayName  string       `xml:"display-name"`
	Revision     Revision     `xml:"revision"`
	Archives     []Archive    `xml:"archives>archive"`
	ChannelRef   ChannelRef   `xml:"channelRef"`
	UsesLicense  UsesLicense  `xml:"uses-license"`
	Dependencies []Dependency `xml:"dependencies>dependency"`
}

type Revision struct {
	Major string `xml:"major"`
	Minor string `xml:"minor"`
	Micro string `xml:"micro"`
}

func (r *Revision) String() string {
	s := r.Major
	if r.Minor != "" {
		s += "." + r.Minor
		if r.Micro != "" {
			s += "." + r.Micro
		}
	}
	return s
}

type Archive struct {
	HostOs   string `xml:"host-os"`
	HostBits uint   `xml:"host-bits"`
	Size     uint64 `xml:"complete>size"`
	Checksum string `xml:"complete>checksum"`
	Url      string `xml:"complete>url"`
}

type ChannelRef struct {
	Ref string `xml:"ref,attr"`
}

type UsesLicense struct {
	Ref string `xml:"ref,attr"`
}

type Dependency struct {
	Path        string   `xml:"path,attr"`
	MinRevision Revision `xml:"min-revision"`
}

func main() {
	baseUrl := "https://dl.google.com/android/repository"
	resp, err := http.Get(baseUrl + "/repository2-1.xml")
	if err != nil {
		log.Fatal(err)
	}
	if resp.StatusCode != 200 {
		log.Fatalf("Unable to get repository2-1.xml: status=%d", resp.StatusCode)
	}

	sdk := Sdk{}
	data, err := ioutil.ReadAll(resp.Body)
	resp.Body.Close()
	if err != nil {
		log.Fatal(err)
	}
	err = xml.Unmarshal(data, &sdk)
	if err != nil {
		log.Fatal(err)
	}

	licenses := map[string]License{}
	for _, license := range sdk.Licenses {
		licenses[license.Id] = license
	}
	channels := map[string]Channel{}
	for _, channel := range sdk.Channels {
		channels[channel.Id] = channel
	}

	for _, pkg := range sdk.RemotePackages {
		fmt.Printf("- path: %s\n", pkg.Path)
		fmt.Printf("  revision: %s\n", pkg.Revision.String())
		fmt.Printf("  channel: %s\n", channels[pkg.ChannelRef.Ref].Name)
		fmt.Printf("  license: %s\n", licenses[pkg.UsesLicense.Ref].Id)
		fmt.Printf("  display: %s\n", pkg.DisplayName)
		fmt.Println("  archives:")
		for _, archive := range pkg.Archives {
			hostOs := "generic"
			if archive.HostOs != "" {
				if archive.HostBits == 0 {
					hostOs = archive.HostOs
				} else {
					hostOs = fmt.Sprintf("%s%d", archive.HostOs, archive.HostBits)
				}
			}
			fmt.Printf("    %s:\n", hostOs)
			fmt.Printf("      url: %s/%s\n", baseUrl, archive.Url)
			fmt.Printf("      size: %d\n", archive.Size)
			fmt.Printf("      checksum: %s\n", archive.Checksum)
		}
		if len(pkg.Dependencies) != 0 {
			fmt.Println("  dependencies:")
			for _, dep := range pkg.Dependencies {
				fmt.Printf("    - %s", dep.Path)
				if dep.MinRevision.Major != "" {
					fmt.Printf(" >= %s", dep.MinRevision.String())
				}
				fmt.Println("")
			}
		}
	}
}
