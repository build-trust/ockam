package log

import (
	"fmt"
	"io"
	"os"

	"github.com/ockam-network/ockam"
)

// LoggerLevel indicates the amout of detail to include in the log
type LoggerLevel uint

const (
	// Quiet will silence the logger
	Quiet LoggerLevel = iota + 1

	// Error only shows error messages
	Error

	// Warn shows warning and error messages
	Warn

	// Notice shows notice, warning and error messages
	Notice

	// Info shows info, notice, warning and error messages
	Info

	// Debug shows debug, info, notice, warning and error messages
	Debug
)

// String returns the string representation of a LogLevel
func (l LoggerLevel) String() string {
	switch l {
	case Error:
		return "Error"
	case Warn:
		return "Warn"
	case Notice:
		return "Notice"
	case Info:
		return "Info"
	case Debug:
		return "Debug"
	default:
		return ""
	}
}

// Option is
type Option func(*Logger)

// Logger is
type Logger struct {
	Level        LoggerLevel
	Writer       io.Writer
	Formatter    LoggerFormatter
	Fields       ockam.Fields
	CurrentLevel LoggerLevel
}

// LoggerFormatter is
type LoggerFormatter interface {
	Format(string) string
	SetLogger(l *Logger)
}

// New returns a new logger
func New(options ...Option) *Logger {
	logger := &Logger{
		Level:  Debug,
		Writer: os.Stderr,
	}

	logger.Formatter = &ConsoleFormatter{
		Logger:      logger,
		Colored:     true,
		Typed:       true,
		TimeStamped: false,
	}

	for _, option := range options {
		option(logger)
	}

	return logger
}

// Level returns an Option
func Level(level LoggerLevel) Option {
	return func(l *Logger) {
		l.Level = level
	}
}

// Formatter returns an Option
func Formatter(formatter LoggerFormatter) Option {
	return func(l *Logger) {
		formatter.SetLogger(l)
		l.Formatter = formatter
	}
}

// Writer returns an Option
func Writer(w io.Writer) Option {
	return func(l *Logger) {
		l.Writer = w
	}
}

// Error writes an error to the log
func (l *Logger) Error(format string, v ...interface{}) {
	l.log(Error, l.Writer, format, v...)
}

// Warn writes a warning to the log
func (l *Logger) Warn(format string, v ...interface{}) {
	l.log(Warn, l.Writer, format, v...)
}

// Notice writes a notice to the log
func (l *Logger) Notice(format string, v ...interface{}) {
	l.log(Notice, l.Writer, format, v...)
}

// Info writes a informative message to the log
func (l *Logger) Info(format string, v ...interface{}) {
	l.log(Info, l.Writer, format, v...)
}

// Debug writes a debug message to the log
func (l *Logger) Debug(format string, v ...interface{}) {
	l.log(Debug, l.Writer, format, v...)
}

// WithFields adds key=value pair fields to the log messagae for structured logging
func (l *Logger) WithFields(fields ockam.Fields) ockam.Logger {
	l.Fields = fields
	return l
}

func (l *Logger) log(current LoggerLevel, writer io.Writer, format string, v ...interface{}) {
	if l.Level >= current {
		l.CurrentLevel = current
		fmt.Fprintf(writer, l.Formatter.Format(format), v...) // nolint: errcheck, gosec
		l.Fields = nil
	}
}
