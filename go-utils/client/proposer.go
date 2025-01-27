package main

import (
	"fmt"
	"math/rand"
	"time"
	. "go-utils"
)

type Proposer struct {
	chainNum int
	msgChans []chan<- string
	interval int
}

func NewProposer(chainNum int, msgChans []chan<- string, interval int) *Proposer {
	pp := &Proposer{
		chainNum: chainNum,
		msgChans: msgChans,
		interval: interval,
	}
	return pp
}

func (pp *Proposer) Run(txNum int) {
	// wls := GenerateWorkloads(txNum, pp.chainNum)
	wls := GenerateWorkloads3(txNum, pp.chainNum, FailureRate)
	for i := 0; i < txNum; i++ {
		for j := 0; j < pp.chainNum-1; j++ {
			pp.msgChans[j] <- wls[i][j]
		}
		if i != txNum-1 {
			time.Sleep(time.Duration(pp.interval) * time.Millisecond)
		}
	}
}

// GenerateWorkloads generate (num X chainNum) json worklaods, each with at least one debit and one credit
func GenerateWorkloads(num int, chainNum int) [][]string {
	logicChainNum := chainNum - 1
	// templates := []string{
	// 	`{"execute_tx":{"fcross_tx":{"tx_id":%d,"operation":{"debit_balance":{"amount":%d}}}}}`,
	// 	`{"execute_tx":{"fcross_tx":{"tx_id":%d,"operation":{"credit_balance":{"amount":%d}}}}}`,
	// }
	templates := []string{
		`{"execute_txs":{"fcross_txs":[{"tx_id":%d,"operation":{"credit_balance":{"amount":%d}}}]}}`,
		`{"execute_txs":{"fcross_txs":[{"tx_id":%d,"operation":{"debit_balance":{"amount":%d}}}]}}`,
	}
	tempLen := len(templates)
	r := rand.New(rand.NewSource(RandSeed))

	workloads := make([][]string, 0, num)
	for i := 1; i <= num; i++ {
		var selection []int
		for true {
			zeroOneRecord := make([]int, 0, logicChainNum)
			for j := 0; j < logicChainNum; j++ {
				zeroOneRecord = append(zeroOneRecord, r.Intn(tempLen))
			}
			s, m := 0, 1
			for j := 0; j < logicChainNum; j++ {
				s += zeroOneRecord[j]
				m *= zeroOneRecord[j]
			}
			// if only one logicalChain, break directly
			// has both 0 and 1
			if (logicChainNum == 1) || (s > 0 && m == 0) {
				selection = zeroOneRecord
				break
			} else {
				continue
			}
		}
		fmt.Printf("%v\n", selection)

		workload := make([]string, 0, logicChainNum)
		for j := 0; j < logicChainNum; j++ {
			workload = append(workload, fmt.Sprintf(templates[selection[j]], i, 10+r.Intn(31)))
		}
		workloads = append(workloads, workload)
	}

	// fmt.Println(workloads)

	return workloads
}

// generate workloads adds to be zero
func GenerateWorkloads2(num int, chainNum int) [][]string {
	logicChainNum := chainNum - 1
	if logicChainNum<=1 {
		panic(fmt.Sprintf("I can't generate workloads for %d logical chains", logicChainNum))
	}

	templates := []string{
		`{"execute_txs":{"fcross_txs":[{"tx_id":%d,"operation":{"credit_balance":{"amount":%d}}}]}}`,
		`{"execute_txs":{"fcross_txs":[{"tx_id":%d,"operation":{"debit_balance":{"amount":%d}}}]}}`,
	}
	r := rand.New(rand.NewSource(RandSeed))

	workloads := make([][]string, 0, num)
	for i := 1; i <= num; i++ {
		workload := make([]string, 0, logicChainNum)
		lastValue := 0
		for j:=0;j<logicChainNum;j++{
			t := 0
			if j<(logicChainNum-1) {
				t = r.Intn(21)-10
				lastValue += -t
			} else {
				t = lastValue
			}
			if t>=0 {
				workload = append(workload, fmt.Sprintf(templates[0], i, t))
			} else {
				workload = append(workload, fmt.Sprintf(templates[1], i, t))
			}
		}
		
		workloads = append(workloads, workload)
	}

	// fmt.Println(workloads)

	return workloads
}

// every sub-transaction has its identical failureRate
func GenerateWorkloads3(num int, chainNum int, failureRate int) [][]string {
	r := rand.New(rand.NewSource(RandSeed))

	numLargeEnough := 300000
	logicChainNum := chainNum - 1
	failureNum := num * failureRate/100
	// failureTxIds[i] indicates the failure txs id in logic-chain i
	failureTxIds := make([][]int, 0, logicChainNum)
	for i := 0; i < logicChainNum; i++ {
		temp := r.Perm(num)[:failureNum]
		for j := 0; j < len(temp); j++ {
			temp[j] += 1
		}
		failureTxIds = append(failureTxIds, temp)
	}
	fmt.Println(failureTxIds)

	templates := []string{
		`{"execute_txs":{"fcross_txs":[{"tx_id":%d,"operation":{"credit_balance":{"amount":%d}}}]}}`,
		`{"execute_txs":{"fcross_txs":[{"tx_id":%d,"operation":{"debit_balance":{"amount":%d}}}]}}`,
	}
	

	workloads := make([][]string, 0, num)
	for i := 1; i <= num; i++ {
		// tx-i
		workload := make([]string, 0, logicChainNum)
		for j:=0;j<logicChainNum;j++{
			if Contain(failureTxIds[j], i) {
				workload = append(workload, fmt.Sprintf(templates[1], i, numLargeEnough))
			} else {
				workload = append(workload, fmt.Sprintf(templates[r.Intn(len(templates))], i, 1+r.Intn(RandScope)))
			}
		}
		workloads = append(workloads, workload)
	}
	fmt.Println(workloads)
	return workloads
}

func Contain(numSlice []int, n int) bool {
	for _,v := range numSlice{
		if v==n {
			return true
		}
	}
	return false
}
