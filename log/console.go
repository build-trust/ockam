package log

import (
	"fmt"
	"runtime"
	"strings"
	"time"
)

const (
	// white is used to set ANSI foreground color white
	white int = 0
	// grey is used to set ANSI foreground color grey
	grey int = 30
	// red is used to set ANSI foreground color red
	red int = 31
	// green is used to set ANSI foreground color green
	green int = 32
	// orange is used to set ANSI foreground color orange
	orange int = 33
)

// ConsoleFormatter formats a log entry for printing to a console
type ConsoleFormatter struct {
	// Logger is a pointer to the logger that will call this ConsoleFormatter
	// The Format function will do nothing if Logger is nil.
	Logger *Logger

	// Colored tells the ConsoleFormatter if the output should be colored.
	// This value is ignored if runtime.GOOS is Windows
	Colored bool

	// Typed tells the ConsoleFormatter if the type of log entry should
	// be prefixed before the rest of the entry.
	Typed bool

	// TimeStamped tells the ConsoleFormatter if a timestamp should be added
	// to the log entry. The timestamp is calculated using time.Now()
	TimeStamped bool

	// TimeStampFormat tells the ConsoleFormatter the format of the timestamp
	// that should be add
	TimeStampFormat string

	builder strings.Builder
}

// SetLogger sets the Logger property of the ConsoleFormatter
// The Format function will do nothing if Logger is nil.
func (f *ConsoleFormatter) SetLogger(l *Logger) {
	f.Logger = l
}

// Format formats a log entry
func (f *ConsoleFormatter) Format(s string) string {
	if f.Logger == nil {
		return s
	}

	f.builder.Reset()
	f.buildType()
	f.buildTimestamp()
	f.builder.WriteString(s) // nolint: errcheck, gosec
	f.buildFields()

	s = f.builder.String()

	if s[len(s)-1] != '\n' {
		s += "\n"
	}

	return f.color(s)
}

func (f *ConsoleFormatter) buildType() {
	if f.Typed {
		t := fmt.Sprintf("[%6s] ", strings.ToUpper(f.Logger.CurrentLevel.String()))
		f.builder.WriteString(t) // nolint: errcheck, gosec
	}
}

func (f *ConsoleFormatter) buildTimestamp() {
	if f.TimeStamped {
		f.builder.WriteString(time.Now().Format(time.RFC3339)) // nolint: errcheck, gosec
		f.builder.WriteRune(' ')                               // nolint: errcheck, gosec
	}
}

func (f *ConsoleFormatter) buildFields() {
	fields := f.Logger.Fields

	if len(fields) > 0 {
		f.builder.WriteRune('\t') // nolint: errcheck, gosec
		for k, v := range fields {
			value, ok := v.(string)
			if !ok {
				value = fmt.Sprint(v)
			}
			f.builder.WriteString(fmt.Sprintf("%s=%s", k, value)) // nolint: errcheck, gosec
		}
	}
}

func (f *ConsoleFormatter) color(s string) string {
	if f.Colored && runtime.GOOS != "windows" {
		switch f.Logger.CurrentLevel {
		case Error:
			return color(s, red)
		case Warn:
			return color(s, orange)
		case Notice:
			return color(s, green)
		case Info:
			return color(s, white)
		case Debug:
			return color(s, grey)
		}
	}
	return s
}

func color(s string, c int) string {
	return fmt.Sprintf("\x1b[0;%dm%s\x1b[0m", c, s)
}
