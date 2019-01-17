package main

import "fmt"

func color(s string, c int) string {
	return fmt.Sprintf("\x1b[0;%dm%s\x1b[0m", c, s)
}

func green(s string) string {
	return color(s, 32)
}

func grey(s string) string {
	return color(s, 30)
}

func white(s string) string {
	return color(s, 0)
}
