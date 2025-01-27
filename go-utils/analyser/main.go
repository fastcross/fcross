package main

import (
	"bufio"
	"bytes"
	"encoding/json"
	"fmt"
	. "go-utils"
	"os"
	"os/exec"
	"strconv"
	"strings"
)

func main() {
	AnalyseLatency()
	AnalyseGasCost()
	AnalyseAbortRate()
}

func AnalyseGasCost() {
	fullProjectDir, err := GetAbsProjectDir()
	if err != nil {
		panic(fmt.Sprintf("function GetAbsProjectDir error: %v", err))
	}

	// user cost
	var gasSubmittingTxs []int
	for i := 1; i < ChainNum; i++ {
		f, err := os.Open(fmt.Sprintf("%s/client_logs/client-%d.txt", fullProjectDir, i))
		if err != nil {
			panic(fmt.Errorf("function os.Open error: %v", err))
		}
		// scan line by line
		scanner := bufio.NewScanner(f)
		sum := 0
		count := 0
		for scanner.Scan() {
			line := scanner.Text()
			if strings.Contains(line, "gas_used") {
				count += 1
				lineAfterSplit := strings.Split(line, "\"")
				amount, err := strconv.Atoi(lineAfterSplit[1])
				if err != nil {
					panic(fmt.Errorf("function strconv.Atoi error: %v", err))
				}
				sum += amount
			}
		}
		if count != CtxNum {
			panic(fmt.Sprintf("we only found %d txs in client_%d.txt, while we need %d", count, i, CtxNum))
		}
		gasSubmittingTxs = append(gasSubmittingTxs, sum)
	}
	fmt.Println(gasSubmittingTxs)
	fmt.Printf("average submitting gas cost: %d\n", Average(gasSubmittingTxs))

	// ibc cost
	// it cost average 25,000 stake to establish ibc connection
	startBalance := 100000000000 - 25000
	exchangeRate := 0.036

	// query the remaining stake of ibc accounts
	var gasHandlingIbc []int
	for i := 1; i < ChainNum; i++ {
		cmd := exec.Command("wasmd", []string{
			"keys", "show", "testkey", "-a",
			"--keyring-backend", "test",
			"--home", fmt.Sprintf("%s/data/ibc-%d", fullProjectDir, i),
		}...)
		var myOutput bytes.Buffer
		cmd.Stdout = &myOutput
		err := cmd.Run()
		if err != nil {
			panic(fmt.Errorf("running cmd %v error: %v", cmd.Args, err))
		}
		testKeyAddress := strings.TrimSpace(myOutput.String())
		cmd = exec.Command("wasmd", []string{
			"query", "bank", "balances", testKeyAddress,
			"--node", fmt.Sprintf("http://127.0.0.1:%d", 26550+i),
			"--home", fmt.Sprintf("%s/data/ibc-%d", fullProjectDir, i),
			"--output", "json",
		}...)
		var myOutput2 bytes.Buffer
		cmd.Stdout = &myOutput2
		err = cmd.Run()
		if err != nil {
			panic(fmt.Errorf("running cmd %v error: %v", cmd.Args, err))
		}
		balanceResult := new(BalanceResult)
		err = json.Unmarshal(myOutput2.Bytes(), balanceResult)
		if err != nil {
			panic(fmt.Errorf("function json.Unmarshal error: %v", err))
		}
		amount := 0
		for _, t := range balanceResult.Balances {
			if t.Denom == "stake" {
				amount, err = strconv.Atoi(t.Amount)
				if err != nil {
					panic(fmt.Errorf("function strconv.Atoi error: %v", err))
				}
				break
			}
		}
		gasHandlingIbc = append(gasHandlingIbc, int(float64(startBalance-amount)/exchangeRate))
	}
	fmt.Println(gasHandlingIbc)
	fmt.Printf("average ibc gas cost: %d\n", Average(gasHandlingIbc))
}

func AnalyseLatency() {
	fullProjectDir, err := GetAbsProjectDir()
	if err != nil {
		panic(fmt.Sprintf("function GetAbsProjectDir error: %v", err))
	}

	// analyse latency
	// queryMsg := `{"my_time_logs":{}}`
	sums := 0
	for i := 1; i < ChainNum; i++ {
		cmd := exec.Command("wasmd", []string{
			"--home", fmt.Sprintf("%s/data/ibc-%d", fullProjectDir, i),
			"query", "wasm", "contract-state", "smart", "wasm14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9s0phg4d", `{"my_time_logs":{}}`,
			"--node", fmt.Sprintf("http://127.0.0.1:%d", 26550+i),
			"--output", "json",
		}...)
		output, err := cmd.CombinedOutput()
		if err != nil {
			panic(fmt.Errorf("function cmd.CombinedOutput error: %v", err))
		}
		tl := new(TimeLog)
		// fmt.Printf("%s\n", output)
		err = json.Unmarshal(output, tl)
		if err != nil {
			panic(fmt.Errorf("function json.Unmarshal error: %v", err))
		}
		// assert
		if len(tl.Data.Logs) != CtxNum {
			panic(fmt.Errorf("expect %d logs, got %d", CtxNum, len(tl.Data.Logs)))
		}
		// unit: ms
		t, err := ReadPropseTime(fullProjectDir + "/" + ProposeTimeFile)
		if err != nil {
			panic(fmt.Errorf("function ReadPropseTime error: %v", err))
		}
		sum := 0
		for j := 0; j < CtxNum; j++ {
			sl := strings.Split(tl.Data.Logs[j], ",")
			executeTime, err := strconv.ParseUint(sl[1][:len(sl[1])-6], 10, 64)
			if err != nil {
				panic(fmt.Errorf("function strconv.ParseUint error: %v", err))
			}
			submitTime := t + uint64(j)*Interval
			endTime, err := strconv.ParseUint(sl[2][:len(sl[2])-6], 10, 64)
			if err != nil {
				panic(fmt.Errorf("function strconv.ParseUint error: %v", err))
			}
			sum += int(endTime - submitTime)
			fmt.Printf("tx_id: %s, start_time: %d, execute_time: %d, end_time: %d\n", sl[0], submitTime/1000, executeTime/1000, endTime/1000)
		}
		sum /= CtxNum
		sums += sum
		fmt.Printf("sum: %d\n", sum)
	}
	fmt.Printf("average sum: %d\n", sums/(ChainNum-1))
}

func AnalyseAbortRate() {
	fullProjectDir, err := GetAbsProjectDir()
	if err != nil {
		panic(fmt.Sprintf("function GetAbsProjectDir error: %v", err))
	}

	queryMsg := `{"closed_votes":{}}`
	cmd := exec.Command("wasmd", []string{
		"query", "wasm", "contract-state", "smart", "wasm14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9s0phg4d", queryMsg,
		"--node", fmt.Sprintf("http://127.0.0.1:%d", 26550),
		"--home", fmt.Sprintf("%s/data/ibc-0", fullProjectDir),
		"--output", "json",
	}...)
	output, err := cmd.CombinedOutput()
	if err != nil {
		panic(fmt.Errorf("function cmd.CombinedOutput error: %v", err))
	}
	outputAsStr := string(output)
	fmt.Printf("output string: %s\n", outputAsStr)
	trueNum := len(strings.Split(outputAsStr, "true")) - 1
	falseNum := len(strings.Split(outputAsStr, "false")) - 1
	fmt.Printf("%d-%d\n", trueNum, falseNum)
}

type TimeLog struct {
	Data struct {
		Logs []string `json:"logs"`
	} `json:"data"`
}

func ReadPropseTime(path string) (uint64, error) {
	content, err := os.ReadFile(path)
	if err != nil {
		return 0, fmt.Errorf("function os.OpenFile error: %v", err)
	}
	contentStr := string(content)
	proposeStartTime, err := strconv.ParseUint(contentStr[:len(contentStr)-6], 10, 64)
	if err != nil {
		return 0, fmt.Errorf("function strconv.ParseUint error: %v", err)
	}
	return proposeStartTime, nil
}

type BalanceResult struct {
	Balances []struct {
		Denom  string `json:"denom"`
		Amount string `json:"amount"`
	} `json:"balances"`
	Pagination struct {
		NextKey interface{} `json:"next_key"`
		Total   string      `json:"total"`
	} `json:"pagination"`
}

func Average(nums []int) int {
	sum := 0
	for _, num := range nums {
		sum += num
	}
	return sum / len(nums)
}
