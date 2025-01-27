package main

import (
	"bytes"
	"fmt"
	"os/exec"
	"strings"
	"sync"
	"time"
	. "go-utils"
)

type Executor struct {
	chainId    int	
	projectDir string
	msgChan    <-chan string
	logChan    chan<- []byte
}

func NewExecutor(chainId int, projectDir string, msgChan <-chan string, logChan chan<- []byte) *Executor {
	ex := &Executor{
		chainId:    chainId,
		projectDir: projectDir,
		msgChan:    msgChan,
		logChan:    logChan,
	}
	return ex
}

func (ex *Executor) Run(txNum int) {
	var wg sync.WaitGroup
	wg.Add(txNum)
	for i := 0; i < txNum; i++ {
		msg := <-ex.msgChan
		args := []string{
			"--home", fmt.Sprintf("%s/data/ibc-%d", ex.projectDir, ex.chainId),
			"tx", "wasm", "execute", "wasm14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9s0phg4d",
			msg,
			"--gas-prices", "0.025stake", "--gas", "20000000", "--gas-adjustment", "1.1",
			"--node", fmt.Sprintf("http://127.0.0.1:%d", 26550+ex.chainId),
			"--chain-id", fmt.Sprintf("ibc-%d", ex.chainId),
			"--from", "user",
			"--broadcast-mode", "block",
			"-y",
			"--keyring-backend", "test",
			"--sequence", fmt.Sprintf("%d", i+2),
		}
		go func() {
			success := ExecuteUntilSuccess("wasmd", args, ex.logChan, MaxTryTimes)
			if !success {
				time.Sleep(3 * time.Second) // we'll wait for log written to file
				panic(fmt.Errorf("function ExecuteUntilSuccess error after trying many time of executing cmd: %s %s", "wasmd", strings.Join(args, " ")))
			}
			wg.Done()
		}()
	}
	wg.Wait()
}

func ExecuteUntilSuccess(cmdName string, cmdArgs []string, logChan chan<- []byte, maxTry int) bool {
	for i := 0; i < maxTry; i++ {
		cmd := exec.Command(cmdName, cmdArgs...)
		output, err := cmd.CombinedOutput()
		if err != nil {
			panic(fmt.Errorf("function cmd.CombinedOutput error when execute cmd: %s %v\nerror: %v\ncombinedOutput:%s", cmdName, cmdArgs, err, output))
		}
		logChan <- append([]byte(fmt.Sprintf("execution logs of cmd (the %dth time) <%s %s>:\n", i+1, cmdName, strings.Join(cmdArgs, " "))), output...)

		// analyse
		if bytes.Contains(output, []byte("fail")) || bytes.Contains(output, []byte("mismatch")) {
			continue
		} else {
			return true
		}
	}
	return false
}
