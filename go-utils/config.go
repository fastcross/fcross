package goutils

import (
	"fmt"
	"os"
	"strings"
)

const ProjectDir = "myfc"
const CtxNum = 100
const ChainNum = 3
const Interval = 300
const MaxTryTimes = 1
const CtxNumPerSubmission = 1 // proposer & analyser need to be modified
const ProposeTimeFile = "proposeTime.txt"
const RandSeed = 114
const FailureRate = 0 // [0, 100]
const RandScope = 10  // default 10

// GetAbsProjectDir 检查路径是否为 myfc 的子目录，并修剪到 myfc 这一层
func GetAbsProjectDir() (string, error) {
	currentDir, err := os.Getwd()
	if err != nil {
		return "", fmt.Errorf("function os.Getwd error: %v", err)
	}

	myfcIndex := strings.Index(currentDir, ProjectDir)
	if myfcIndex == -1 {
		return "", fmt.Errorf("the binary should be run in the project dir %s", ProjectDir)
	}

	return currentDir[:myfcIndex+len(ProjectDir)], nil
}
