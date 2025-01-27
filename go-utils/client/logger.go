package main

import (
	"fmt"
	"os"
)

type Logger struct {
	logChan <-chan []byte
	logFile string
}

func NewLogger(logChan <-chan []byte, logFile string) *Logger {
	lg := &Logger{
		logChan: logChan,
		logFile: logFile,
	}
	return lg
}

func (lg *Logger) Run() {
	f, err := os.OpenFile(lg.logFile, os.O_CREATE|os.O_TRUNC|os.O_WRONLY, 0644)
	if err != nil {
		panic(fmt.Errorf("function os.OpenFile error: %v", err))
	}
	for newLog := range lg.logChan {
		_, err := f.Write(newLog)
		if err != nil {
			panic(fmt.Errorf("function f.Write error: %v", err))
		}
	}
	_ = f.Close()
}
