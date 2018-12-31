package ockam

// Fields is
type Fields map[string]interface{}

// Logger is an interface for Logging.
type Logger interface {
	Error(format string, v ...interface{})
	Warn(format string, v ...interface{})
	Notice(format string, v ...interface{})
	Info(format string, v ...interface{})
	Debug(format string, v ...interface{})
	WithFields(fields Fields) Logger
}

// Version returns the current version of Ockam
func Version() string {
	version := "0.2.0-develop"
	return version
}
