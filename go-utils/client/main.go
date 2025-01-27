package main

import (
	"fmt"
	. "go-utils"
	"os"
	"sync"
	"time"
)

func main() {
	fullProjectDir, err := GetAbsProjectDir()
	if err != nil {
		panic(fmt.Sprintf("function GetAbsfullProjectDir error: %v", err))
	}

	// prepare msg chans
	sendOnlyChans := make([]chan<- string, ChainNum-1)
	receiveOnlyChans := make([]<-chan string, ChainNum-1)
	for i := 0; i < ChainNum-1; i++ {
		ch := make(chan string)
		sendOnlyChans[i] = ch
		receiveOnlyChans[i] = ch
	}
	// prepare log chans
	logChans := make([]chan []byte, ChainNum-1)
	for i := 0; i < ChainNum-1; i++ {
		ch := make(chan []byte)
		logChans[i] = ch
	}

	// start logger
	for i := 1; i < ChainNum; i++ {
		lg := NewLogger(logChans[i-1], fmt.Sprintf("%s/client_logs/client-%d.txt", fullProjectDir, i))
		go func() {
			lg.Run()
		}()
	}

	// start 1 proposer & c-1 executor
	var wg sync.WaitGroup
	wg.Add(ChainNum)

	pp := NewProposer(ChainNum, sendOnlyChans, Interval)
	err = RecordPropseTime(fullProjectDir+"/"+ProposeTimeFile)
	if err!=nil {
		panic(fmt.Sprintf("function RecordPropseTime error: %v", err))
	}
	go func() {
		pp.Run(CtxNum)
		wg.Done()
	}()

	for i := 1; i < ChainNum; i++ {
		ex := NewExecutor(i, fullProjectDir, receiveOnlyChans[i-1], logChans[i-1])
		go func() {
			ex.Run(CtxNum)
			wg.Done()
		}()
	}

	wg.Wait()

	// close logChans to end all logger
	for _, logChan := range logChans {
		close(logChan)
	}
	// waiting for writing logs done
	time.Sleep(3 * time.Second)
	fmt.Println("all done!")
}

func RecordPropseTime(path string) error{
	f, err := os.OpenFile(path, os.O_CREATE|os.O_TRUNC|os.O_WRONLY, 0644)
	if err != nil {
		return fmt.Errorf("function os.OpenFile error: %v", err)
	}
	_, err = f.WriteString(fmt.Sprintf("%d", time.Now().UnixNano()))
	if err != nil {
		return fmt.Errorf("function f.WriteString error: %v", err)
	}
	_ = f.Close()
	return nil
}
